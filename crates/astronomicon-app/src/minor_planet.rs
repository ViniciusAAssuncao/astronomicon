use crate::ephemeris::resolve_system_positions;
use crate::error::AppResult;
use crate::hierarchy::find_parent_star;
use astronomicon_core::domain::{MinorPlanet, SpectralType};
use astronomicon_core::error::DomainError;
use astronomicon_core::math::cometary::{
    coma_radius, cometary_gas_production_rate, cometary_tail_structure, sublimation_equilibrium,
    thermal_gas_expansion_speed, CometaryVolatile,
};
use astronomicon_core::math::gravity::gravitational_parameter;
use astronomicon_core::math::minor_planet::{
    bulk_density, critical_rotation_period, equivalent_spherical_radius,
    grain_density_by_spectral_type, triaxial_ellipsoid_surface_area, triaxial_ellipsoid_volume,
};
use astronomicon_core::math::radiometry::{
    escape_velocity, orbital_irradiance, stellar_luminosity,
};
use astronomicon_core::math::stellar_wind::{
    reimers_mass_loss_rate, stellar_wind_density, stellar_wind_dynamic_pressure,
    terminal_wind_speed,
};
use astronomicon_core::units::{Density, Duration, Irradiance, Length, MassRate};
use astronomicon_db::repositories::minor_planet_repository;
use astronomicon_db::SqlitePool;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RubblePileDiagnostic {
    pub grain_density: Density,
    pub bulk_density: Density,
    pub macroporosity: f64,
    pub critical_rotation_period: Duration,
    pub is_centrifugal_shedding: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CometaryActivityDiagnostic {
    pub is_active: bool,
    pub water_outgassing_rate: MassRate,
    pub co2_outgassing_rate: MassRate,
    pub total_gas_production_rate: MassRate,
    pub coma_radius: Length,
    pub ion_tail_length: Length,
    pub dust_tail_length: Length,
    pub ion_tail_drag_force_n: f64,
    pub dust_radiation_force_n: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MinorPlanetDiagnostic {
    pub equivalent_spherical_radius: Length,
    pub volume_m3: f64,
    pub surface_area_m2: f64,
    pub rubble_pile: RubblePileDiagnostic,
    pub cometary_activity: Option<CometaryActivityDiagnostic>,
}

pub async fn resolve_minor_planet_diagnostics(
    pool: &SqlitePool,
    minor_planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<MinorPlanetDiagnostic> {
    let row = minor_planet_repository::get_by_id(pool, &minor_planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "minor_planet_id".to_string(),
            reason: format!("minor planet '{}' not found", minor_planet_id),
        })?;
    let minor_planet = MinorPlanet::try_from(row)?;

    let spectral_type = minor_planet.spectral_type();
    let macroporosity = minor_planet.macroporosity().unwrap_or(0.0);
    let grain_rho = grain_density_by_spectral_type(spectral_type);
    let bulk_rho = bulk_density(grain_rho, macroporosity);
    let crit_rot = critical_rotation_period(bulk_rho);

    let is_centrifugal_shedding = minor_planet
        .rotation_period()
        .map(|rot| rot.value() < crit_rot.value())
        .unwrap_or(false);

    let rubble_pile = RubblePileDiagnostic {
        grain_density: grain_rho,
        bulk_density: bulk_rho,
        macroporosity,
        critical_rotation_period: crit_rot,
        is_centrifugal_shedding,
    };

    let (axis_a, axis_b, axis_c) = match (
        minor_planet.axis_a(),
        minor_planet.axis_b(),
        minor_planet.axis_c(),
    ) {
        (Some(a), Some(b), Some(c)) => (a, b, c),
        _ => {
            let vol_est = minor_planet.mass().value() / bulk_rho.value().max(1.0);
            let r_est = Length::new((3.0 * vol_est / (4.0 * PI)).cbrt());
            (
                minor_planet.axis_a().unwrap_or(r_est),
                minor_planet.axis_b().unwrap_or(r_est),
                minor_planet.axis_c().unwrap_or(r_est),
            )
        }
    };

    let volume_m3 = triaxial_ellipsoid_volume(axis_a, axis_b, axis_c);
    let surface_area_m2 = triaxial_ellipsoid_surface_area(axis_a, axis_b, axis_c);
    let equivalent_spherical_radius = equivalent_spherical_radius(axis_a, axis_b, axis_c);

    let cometary_activity = match find_parent_star(pool, minor_planet.orbital_parent()).await {
        Ok(star) => {
            let star_temp = star.effective_temperature();
            let star_radius = star.radius();
            let star_sys_id = star.star_system_id();

            match (star_temp, star_radius, star_sys_id) {
                (Some(t_star), Some(r_star), Some(sys_id)) => {
                    let total_epoch = universe_epoch + at_epoch;
                    let positions = resolve_system_positions(pool, sys_id, total_epoch).await?;

                    if let (Some(&pos_mp), Some(&pos_star)) = (
                        positions.get(&minor_planet.id()),
                        positions.get(&star.id()),
                    ) {
                        let orbital_dist = (pos_mp - pos_star).magnitude();
                        let star_lum = stellar_luminosity(r_star, t_star);
                        let top_irradiance = orbital_irradiance(star_lum, orbital_dist);

                        let mu_star = gravitational_parameter(star.mass());
                        let v_esc = escape_velocity(mu_star, r_star);
                        let m_dot = reimers_mass_loss_rate(star_lum, r_star, star.mass(), 1.0);
                        let v_inf = terminal_wind_speed(v_esc, 1.0);
                        let rho_sw = stellar_wind_density(m_dot, v_inf, orbital_dist);
                        let p_dyn = stellar_wind_dynamic_pressure(rho_sw, v_inf);

                        let bond_albedo = minor_planet.bond_albedo().unwrap_or(0.04);
                        let active_fraction = match spectral_type {
                            SpectralType::C | SpectralType::D | SpectralType::P => 0.05,
                            _ => 0.005,
                        };

                        let (rate_h2o, _) = cometary_gas_production_rate(
                            surface_area_m2,
                            active_fraction,
                            top_irradiance,
                            bond_albedo,
                            CometaryVolatile::Water,
                        );

                        let (rate_co2, _) = cometary_gas_production_rate(
                            surface_area_m2,
                            active_fraction * 0.25,
                            top_irradiance,
                            bond_albedo,
                            CometaryVolatile::CarbonDioxide,
                        );

                        let total_gas = MassRate::new(rate_h2o.value() + rate_co2.value());

                        if total_gas.value() > 1.0e-6 {
                            let (t_eq_h2o, _) = sublimation_equilibrium(
                                Irradiance::new(top_irradiance.value() * 0.25),
                                bond_albedo,
                                0.9,
                                CometaryVolatile::Water,
                            );
                            let v_exp =
                                thermal_gas_expansion_speed(t_eq_h2o, CometaryVolatile::Water);
                            let r_coma = coma_radius(total_gas, v_exp, p_dyn, top_irradiance);
                            let tail = cometary_tail_structure(
                                total_gas,
                                1.0,
                                v_exp,
                                r_coma,
                                p_dyn,
                                v_inf,
                                top_irradiance,
                                orbital_dist,
                            );

                            Some(CometaryActivityDiagnostic {
                                is_active: true,
                                water_outgassing_rate: rate_h2o,
                                co2_outgassing_rate: rate_co2,
                                total_gas_production_rate: total_gas,
                                coma_radius: r_coma,
                                ion_tail_length: tail.ion_tail_length,
                                dust_tail_length: tail.dust_tail_length,
                                ion_tail_drag_force_n: tail.ion_tail_drag_force_n,
                                dust_radiation_force_n: tail.dust_radiation_force_n,
                            })
                        } else {
                            Some(CometaryActivityDiagnostic {
                                is_active: false,
                                water_outgassing_rate: rate_h2o,
                                co2_outgassing_rate: rate_co2,
                                total_gas_production_rate: total_gas,
                                coma_radius: Length::new(0.0),
                                ion_tail_length: Length::new(0.0),
                                dust_tail_length: Length::new(0.0),
                                ion_tail_drag_force_n: 0.0,
                                dust_radiation_force_n: 0.0,
                            })
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }
        Err(_) => None,
    };

    Ok(MinorPlanetDiagnostic {
        equivalent_spherical_radius,
        volume_m3,
        surface_area_m2,
        rubble_pile,
        cometary_activity,
    })
}
