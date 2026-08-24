use crate::units::{Acceleration, Density, HeatFlux, Length, Pressure, Temperature};

pub fn lithospheric_base_pressure(
    surface_gravity: Acceleration,
    mantle_density: Density,
    lithosphere_thickness: Length,
) -> Pressure {
    let g = surface_gravity.value();
    let rho = mantle_density.value();
    let z = lithosphere_thickness.value();

    if g <= 0.0 || rho <= 0.0 || z <= 0.0 || !g.is_finite() || !rho.is_finite() || !z.is_finite() {
        return Pressure::new(0.0);
    }

    Pressure::new(rho * g * z)
}

pub fn depressed_solidus_temperature(
    dry_solidus: Temperature,
    mantle_hydration_fraction: f64,
) -> Temperature {
    let t_dry = dry_solidus.value();
    if !t_dry.is_finite() || t_dry <= 0.0 {
        return Temperature::new(0.0);
    }

    let h = mantle_hydration_fraction.clamp(0.0, 1.0);
    if h <= 0.0 {
        return dry_solidus;
    }

    let delta_t_max = 450.0;
    let delta_t = delta_t_max * (1.0 - (-25.0 * h).exp());
    let t_wet = (t_dry - delta_t).max(273.15);

    Temperature::new(t_wet)
}

pub fn mantle_potential_temperature(
    solidus_temperature: Temperature,
    convective_heat_flux: HeatFlux,
) -> Temperature {
    let t_sol = solidus_temperature.value();
    let q = convective_heat_flux.value();

    if !t_sol.is_finite() || t_sol <= 0.0 {
        return Temperature::new(0.0);
    }

    if !q.is_finite() || q <= 0.0 {
        return solidus_temperature;
    }

    let q_ref = 0.04;
    let factor = (q / q_ref).sqrt().clamp(0.0, 4.0);
    let delta_t = 250.0 * factor;

    Temperature::new(t_sol + delta_t)
}

pub fn decompression_melting_temperature(
    base_temperature: Temperature,
    surface_gravity: Acceleration,
    lithosphere_thickness: Length,
    specific_heat_capacity: f64,
    thermal_expansion: f64,
) -> Temperature {
    let t_base = base_temperature.value();
    let g = surface_gravity.value();
    let z_l = lithosphere_thickness.value();
    let cp = specific_heat_capacity;
    let alpha = thermal_expansion;

    if t_base <= 0.0
        || g <= 0.0
        || z_l <= 0.0
        || cp <= 0.0
        || alpha <= 0.0
        || !t_base.is_finite()
        || !g.is_finite()
        || !z_l.is_finite()
        || !cp.is_finite()
        || !alpha.is_finite()
    {
        return base_temperature;
    }

    let adiabatic_gradient = (alpha * g * t_base) / cp;
    let z_ascent = z_l * 0.9;
    let t_ascent = t_base - adiabatic_gradient * z_ascent;

    Temperature::new(t_ascent.max(0.0))
}

pub fn partial_melt_fraction(
    local_temperature: Temperature,
    solidus: Temperature,
    liquidus: Temperature,
) -> f64 {
    let t = local_temperature.value();
    let t_sol = solidus.value();
    let t_liq = liquidus.value();

    if !t.is_finite() || !t_sol.is_finite() || !t_liq.is_finite() || t_liq <= t_sol || t <= t_sol {
        return 0.0;
    }

    if t >= t_liq {
        return 1.0;
    }

    ((t - t_sol) / (t_liq - t_sol)).clamp(0.0, 1.0)
}

pub fn cryovolcanic_melt_fraction(
    surface_temperature: Temperature,
    solvent_melting_point: Temperature,
    geothermal_heat_flux: HeatFlux,
    solute_fraction: f64,
    ice_thickness: Length,
    ice_thermal_conductivity: f64,
) -> (f64, Temperature) {
    let t_surf = surface_temperature.value();
    let t_melt_base = solvent_melting_point.value();
    let q_geo = geothermal_heat_flux.value();
    let w = solute_fraction.clamp(0.0, 0.999);
    let z_ice = ice_thickness.value();
    let k = ice_thermal_conductivity;

    if t_melt_base <= 0.0 || !t_surf.is_finite() || !t_melt_base.is_finite() {
        return (0.0, Temperature::new(0.0));
    }

    let depression = 60.0 * (1.0 - (-10.0 * w).exp());
    let t_freeze = (t_melt_base - depression).max(50.0);

    let basal_temp = if q_geo > 0.0
        && k > 0.0
        && z_ice > 0.0
        && q_geo.is_finite()
        && k.is_finite()
        && z_ice.is_finite()
    {
        t_surf + (q_geo * z_ice) / k
    } else {
        t_surf
    };

    if basal_temp <= t_freeze {
        (0.0, Temperature::new(basal_temp))
    } else {
        let delta_t_range = 30.0;
        let frac = ((basal_temp - t_freeze) / delta_t_range).clamp(0.0, 1.0);
        (frac, Temperature::new(basal_temp))
    }
}
