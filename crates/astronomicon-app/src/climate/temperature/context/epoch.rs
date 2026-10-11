use super::{ClimateBodyInputs, SurfaceTemperatureInputs};
use crate::climate::circulation::PlanetaryCirculationDiagnostic;
use crate::climate::temperature::resolve_top_of_atmosphere_irradiance;
use crate::error::AppResult;
use astronomicon_core::error::DomainError;
use astronomicon_core::math::circulation::{
    circulation_cells_per_hemisphere, equatorial_rossby_deformation_radius, rhines_scale,
};
use astronomicon_core::math::climate::{
    atmospheric_column_heat_capacity, blended_local_temperature, combined_column_heat_capacity,
    combined_thermal_redistribution_efficiency, local_radiative_equilibrium_temperature,
    solar_declination, thermal_redistribution_efficiency,
};
use astronomicon_core::math::gravity::{
    combined_gravitational_parameter, gravitational_parameter, surface_gravity,
};
use astronomicon_core::math::kepler::true_anomaly_at_epoch;
use astronomicon_core::math::rotation::{
    angular_velocity_from_rotation_period, rossby_beta_parameter,
};
use astronomicon_core::units::{Angle, Duration, Irradiance, Length, Pressure, Speed, Temperature};
use astronomicon_db::SqlitePool;
use std::f64::consts::PI;

pub(super) struct EpochClimateData {
    pub(super) global_mean: Temperature,
    pub(super) circulation: PlanetaryCirculationDiagnostic,
    pub(super) surface: SurfaceTemperatureInputs,
}

impl EpochClimateData {
    pub(super) async fn resolve(
        pool: &SqlitePool,
        body: &ClimateBodyInputs,
        universe_epoch: Duration,
        at_epoch: Duration,
    ) -> AppResult<Self> {
        let planet = &body.planet;
        let star = &body.star;
        let top_irradiance =
            resolve_top_of_atmosphere_irradiance(pool, planet, star, universe_epoch, at_epoch)
                .await?;
        let greenhouse = body
            .atmosphere
            .as_ref()
            .map(|atmosphere| atmosphere.greenhouse_effect())
            .unwrap_or_else(|| Temperature::new(0.0));
        let bond_albedo = planet.bond_albedo().unwrap_or(0.3);
        let effective_albedo = match body.hydrosphere.as_ref() {
            Some(hydrosphere) => {
                let base_eq = local_radiative_equilibrium_temperature(
                    Irradiance::new(top_irradiance.value() * 0.25),
                    bond_albedo,
                );
                let base_surface_temp = base_eq + greenhouse;
                let pressure = body
                    .atmosphere
                    .as_ref()
                    .map(|atmosphere| atmosphere.surface_pressure())
                    .unwrap_or_else(|| Pressure::new(0.0));
                let initial_state = hydrosphere.matter_state(base_surface_temp, pressure)?;
                hydrosphere.dynamic_albedo(bond_albedo, initial_state)?
            }
            None => bond_albedo,
        };
        let global_mean = local_radiative_equilibrium_temperature(
            Irradiance::new(top_irradiance.value() * 0.25),
            effective_albedo,
        ) + greenhouse;

        let orbital_elements =
            planet
                .orbital_elements()
                .ok_or_else(|| DomainError::InvalidInvariant {
                    field: "orbital_elements".to_string(),
                    reason: "planet does not have orbital elements".to_string(),
                })?;
        let total_epoch = universe_epoch + at_epoch;
        let mu = combined_gravitational_parameter(planet.mass(), star.mass());
        let true_anomaly = true_anomaly_at_epoch(&orbital_elements, mu, total_epoch)?;
        let obliquity = planet.obliquity().unwrap_or_else(|| Angle::new(0.0));
        let solstice_true_anomaly = planet
            .solstice_true_anomaly()
            .unwrap_or_else(|| Angle::new(0.0));
        let declination = solar_declination(
            obliquity,
            orbital_elements.argument_of_periapsis(),
            solstice_true_anomaly,
            true_anomaly,
        );
        let surface = SurfaceTemperatureInputs {
            declination,
            top_irradiance,
            greenhouse,
            bond_albedo,
        };

        let radius = planet
            .equatorial_radius()
            .ok_or_else(|| DomainError::InvalidInvariant {
                field: "equatorial_radius".to_string(),
                reason: "planet does not have equatorial radius".to_string(),
            })?;
        let rot_period = planet
            .rotation_period()
            .unwrap_or_else(|| Duration::new(86400.0));
        let omega = angular_velocity_from_rotation_period(rot_period);
        let beta_eq = rossby_beta_parameter(omega, Angle::new(0.0), radius);
        let gravity = surface_gravity(gravitational_parameter(planet.mass()), radius);
        let (scale_height, atmosphere_heat_capacity) = match body.atmosphere.as_ref() {
            Some(atmosphere) => {
                let height = atmosphere.scale_height(gravity, global_mean)?;
                let cp = atmosphere.mean_specific_heat_capacity()?;
                let heat_capacity =
                    atmospheric_column_heat_capacity(atmosphere.surface_pressure(), gravity, cp);
                (height, heat_capacity)
            }
            None => (Length::new(8500.0), 0.0),
        };
        let rossby_radius = equatorial_rossby_deformation_radius(gravity, scale_height, beta_eq);
        let thermal_inertia = planet.thermal_inertia().unwrap_or(0.0);
        let equator_temperature = blended_local_temperature(
            global_mean,
            surface.local_surface_temperature(Angle::new(0.0)),
            thermal_inertia,
        );
        let pole_temperature = blended_local_temperature(
            global_mean,
            surface.local_surface_temperature(Angle::new(PI / 2.0)),
            thermal_inertia,
        );
        let delta_temperature = (equator_temperature.value() - pole_temperature.value())
            .abs()
            .max(1.0);
        let characteristic_speed = Speed::new(
            (gravity.value() * scale_height.value() * (delta_temperature / global_mean.value()))
                .sqrt(),
        );
        let rhines = rhines_scale(characteristic_speed, beta_eq);
        let cells = circulation_cells_per_hemisphere(radius, rhines);
        let (efficiency, column_heat_capacity) = match body.hydrosphere.as_ref() {
            Some(hydrosphere) => {
                let ocean_capacity = hydrosphere.oceanic_column_heat_capacity()?;
                let coverage = hydrosphere.surface_coverage_fraction();
                let efficiency = combined_thermal_redistribution_efficiency(
                    atmosphere_heat_capacity,
                    ocean_capacity,
                    coverage,
                    cells,
                );
                let capacity = combined_column_heat_capacity(
                    atmosphere_heat_capacity,
                    ocean_capacity,
                    coverage,
                );
                (efficiency, capacity)
            }
            None => (
                thermal_redistribution_efficiency(atmosphere_heat_capacity, cells),
                atmosphere_heat_capacity,
            ),
        };
        let circulation = PlanetaryCirculationDiagnostic {
            angular_velocity: omega,
            equatorial_beta: beta_eq,
            rossby_deformation_radius: rossby_radius,
            rhines_scale: rhines,
            circulation_cells: cells,
            column_heat_capacity,
            thermal_redistribution_efficiency: efficiency,
        };
        Ok(Self {
            global_mean,
            circulation,
            surface,
        })
    }
}
