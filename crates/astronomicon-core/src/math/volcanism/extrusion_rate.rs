use crate::domain::{PlanetKind, TectonicRegime};
use crate::math::volcanism::magma_properties::buoyancy_overpressure;
use crate::units::constants::{SILICATE_LATENT_HEAT_OF_FUSION, SILICATE_MELT_SPECIFIC_HEAT};
use crate::units::{
    Acceleration, Density, HeatFlux, Length, MassRate, Pressure, Speed, Temperature,
};
use std::f64::consts::PI;

pub fn heat_pipe_extrusion_rate(
    total_surface_heat_flux: HeatFlux,
    planet_radius: Length,
    latent_heat_of_fusion: f64,
    specific_heat_capacity: f64,
    magma_temperature: Temperature,
    surface_temperature: Temperature,
) -> MassRate {
    let q = total_surface_heat_flux.value();
    let r = planet_radius.value();
    let l_f = if latent_heat_of_fusion > 0.0 && latent_heat_of_fusion.is_finite() {
        latent_heat_of_fusion
    } else {
        SILICATE_LATENT_HEAT_OF_FUSION
    };
    let cp = if specific_heat_capacity > 0.0 && specific_heat_capacity.is_finite() {
        specific_heat_capacity
    } else {
        SILICATE_MELT_SPECIFIC_HEAT
    };
    let t_m = magma_temperature.value();
    let t_s = surface_temperature.value();

    if q <= 0.0
        || r <= 0.0
        || l_f <= 0.0
        || cp <= 0.0
        || !q.is_finite()
        || !r.is_finite()
        || !l_f.is_finite()
        || !cp.is_finite()
    {
        return MassRate::new(0.0);
    }

    let area = 4.0 * PI * r * r;
    let total_heat_loss = q * area;
    let delta_t = (t_m - t_s).max(0.0);
    let enthalpy_per_kg = l_f + cp * delta_t;

    if enthalpy_per_kg <= 0.0 {
        return MassRate::new(0.0);
    }

    MassRate::new(total_heat_loss / enthalpy_per_kg)
}

pub fn plate_tectonics_extrusion_rate(
    plate_velocity: Speed,
    plate_count: u32,
    planet_radius: Length,
    lithosphere_thickness: Length,
    mantle_density: Density,
    melt_fraction: f64,
    mantle_hydration: f64,
) -> MassRate {
    let v = plate_velocity.value();
    let r = planet_radius.value();
    let z_l = lithosphere_thickness.value();
    let rho = mantle_density.value();
    let phi = melt_fraction.clamp(0.0, 1.0);
    let h_frac = mantle_hydration.clamp(0.0, 1.0);

    if v <= 0.0
        || r <= 0.0
        || z_l <= 0.0
        || rho <= 0.0
        || phi <= 0.0
        || !v.is_finite()
        || !r.is_finite()
        || !z_l.is_finite()
        || !rho.is_finite()
    {
        return MassRate::new(0.0);
    }

    let n_plates = plate_count.max(2) as f64;
    let ridge_length = n_plates.sqrt() * PI * r * 0.5;
    let spreading_area_rate = ridge_length * v;
    let melt_column_thickness = (z_l * phi).min(r * 0.1);
    let ridge_mass_rate = rho * spreading_area_rate * melt_column_thickness * phi;

    let subduction_length = ridge_length;
    let convergence_area_rate = subduction_length * v;
    let subduction_factor = (h_frac / 0.02).clamp(0.0, 3.0);
    let subduction_mass_rate =
        rho * convergence_area_rate * melt_column_thickness * 0.25 * subduction_factor;

    MassRate::new(ridge_mass_rate + subduction_mass_rate)
}

pub fn stagnant_lid_extrusion_rate(
    cmb_heat_flux: HeatFlux,
    core_radius: Length,
    planet_radius: Length,
    lithosphere_thickness: Length,
    mantle_density: Density,
    lithosphere_yield_strength: Pressure,
    buoyancy_overpressure: Pressure,
    latent_heat_of_fusion: f64,
    specific_heat_capacity: f64,
    delta_temperature: Temperature,
) -> MassRate {
    let q_cmb = cmb_heat_flux.value();
    let r_c = core_radius.value();
    let r_p = planet_radius.value();
    let z_l = lithosphere_thickness.value();
    let _rho = mantle_density.value();
    let sigma_y = lithosphere_yield_strength.value();
    let delta_p = buoyancy_overpressure.value();
    let l_f = if latent_heat_of_fusion > 0.0 && latent_heat_of_fusion.is_finite() {
        latent_heat_of_fusion
    } else {
        SILICATE_LATENT_HEAT_OF_FUSION
    };
    let cp = if specific_heat_capacity > 0.0 && specific_heat_capacity.is_finite() {
        specific_heat_capacity
    } else {
        SILICATE_MELT_SPECIFIC_HEAT
    };
    let dt = delta_temperature.value().max(10.0);

    if q_cmb <= 0.0
        || r_c <= 0.0
        || r_p <= 0.0
        || z_l <= 0.0
        || l_f <= 0.0
        || cp <= 0.0
        || !q_cmb.is_finite()
        || !r_c.is_finite()
        || !r_p.is_finite()
        || !z_l.is_finite()
        || !l_f.is_finite()
        || !cp.is_finite()
    {
        return MassRate::new(0.0);
    }

    let area_cmb = 4.0 * PI * r_c * r_c;
    let plume_power = q_cmb * area_cmb * 0.2;
    let enthalpy = l_f + cp * dt;
    let potential_melt_rate = plume_power / enthalpy;

    let stress_ratio = if sigma_y > 0.0 {
        (delta_p / sigma_y).max(0.0)
    } else {
        1.0
    };
    let penetration_factor = stress_ratio.powf(1.5).clamp(0.01, 1.0);
    let thickness_attenuation = r_p / (r_p + 8.0 * z_l);

    MassRate::new(potential_melt_rate * penetration_factor * thickness_attenuation)
}

pub fn global_magma_extrusion_rate(
    regime: TectonicRegime,
    kind: PlanetKind,
    total_surface_heat_flux: HeatFlux,
    cmb_heat_flux: HeatFlux,
    planet_radius: Length,
    core_radius: Length,
    lithosphere_thickness: Length,
    plate_velocity: Speed,
    plate_count: u32,
    mantle_density: Density,
    crust_density: Density,
    magma_density: Density,
    surface_gravity: Acceleration,
    lithosphere_yield_strength: Pressure,
    melt_fraction: f64,
    mantle_hydration: f64,
    latent_heat_of_fusion: f64,
    specific_heat_capacity: f64,
    magma_temperature: Temperature,
    surface_temperature: Temperature,
) -> MassRate {
    match regime {
        TectonicRegime::HeatPipe => heat_pipe_extrusion_rate(
            total_surface_heat_flux,
            planet_radius,
            latent_heat_of_fusion,
            specific_heat_capacity,
            magma_temperature,
            surface_temperature,
        ),
        TectonicRegime::PlateTectonics => plate_tectonics_extrusion_rate(
            plate_velocity,
            plate_count,
            planet_radius,
            lithosphere_thickness,
            mantle_density,
            melt_fraction,
            mantle_hydration,
        ),
        TectonicRegime::StagnantLid => {
            let overpressure = buoyancy_overpressure(
                crust_density,
                magma_density,
                surface_gravity,
                lithosphere_thickness,
            );
            let dt = Temperature::new(
                (magma_temperature.value() - surface_temperature.value()).max(0.0),
            );
            stagnant_lid_extrusion_rate(
                cmb_heat_flux,
                core_radius,
                planet_radius,
                lithosphere_thickness,
                mantle_density,
                lithosphere_yield_strength,
                overpressure,
                latent_heat_of_fusion,
                specific_heat_capacity,
                dt,
            )
        }
        TectonicRegime::IceTectonics => {
            if matches!(
                kind,
                PlanetKind::IcyBody | PlanetKind::IceGiant | PlanetKind::DwarfPlanet
            ) {
                plate_tectonics_extrusion_rate(
                    plate_velocity,
                    plate_count,
                    planet_radius,
                    lithosphere_thickness,
                    mantle_density,
                    melt_fraction,
                    mantle_hydration,
                )
            } else {
                MassRate::new(0.0)
            }
        }
        TectonicRegime::Inactive => MassRate::new(0.0),
    }
}
