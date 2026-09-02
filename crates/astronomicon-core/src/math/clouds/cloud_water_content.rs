use crate::units::{Density, Length, Temperature, TemperatureGradient};
use crate::units::constants::DEFAULT_MIXED_PHASE_DEPTH_METERS;

pub fn adiabatic_condensate_density(
    air_density: Density,
    base_mixing_ratio: f64,
    local_saturation_mixing_ratio: f64,
) -> Density {
    let rho_air = air_density.value();
    if !rho_air.is_finite()
        || rho_air <= 0.0
        || !base_mixing_ratio.is_finite()
        || !local_saturation_mixing_ratio.is_finite()
    {
        return Density::new(0.0);
    }

    let delta_r = (base_mixing_ratio - local_saturation_mixing_ratio).max(0.0);
    let rho_condensate = rho_air * delta_r;

    if !rho_condensate.is_finite() || rho_condensate <= 0.0 {
        Density::new(0.0)
    } else {
        Density::new(rho_condensate)
    }
}

pub fn freezing_level_altitude(
    surface_temperature: Temperature,
    freezing_point: Temperature,
    lapse_rate: TemperatureGradient,
) -> Option<Length> {
    let t_s = surface_temperature.value();
    let t_f = freezing_point.value();
    let gamma = lapse_rate.value();

    if !t_s.is_finite()
        || !t_f.is_finite()
        || !gamma.is_finite()
        || gamma <= 0.0
        || t_s <= 0.0
        || t_f <= 0.0
    {
        return None;
    }

    if t_s <= t_f {
        return Some(Length::new(0.0));
    }

    let z = (t_s - t_f) / gamma;
    if !z.is_finite() || z < 0.0 {
        None
    } else {
        Some(Length::new(z))
    }
}

pub fn ice_fraction_with_transition(
    altitude: Length,
    freezing_level_altitude: Length,
    transition_depth: Length,
) -> f64 {
    let z = altitude.value();
    let z_f = freezing_level_altitude.value();
    let dz = transition_depth.value().max(1.0);

    if !z.is_finite() || !z_f.is_finite() || !dz.is_finite() {
        return 0.0;
    }

    let half_dz = dz * 0.5;
    let z_bottom = z_f - half_dz;
    let z_top = z_f + half_dz;

    if z <= z_bottom {
        0.0
    } else if z >= z_top {
        1.0
    } else {
        let x = (z - z_bottom) / dz;
        x.clamp(0.0, 1.0)
    }
}

pub fn ice_fraction_at_altitude(
    altitude: Length,
    freezing_level_altitude: Length,
) -> f64 {
    ice_fraction_with_transition(
        altitude,
        freezing_level_altitude,
        Length::new(DEFAULT_MIXED_PHASE_DEPTH_METERS),
    )
}

pub fn mixed_phase_layer_bounds(
    freezing_level_altitude: Length,
    transition_depth: Length,
) -> (Length, Length) {
    let z_f = freezing_level_altitude.value();
    let half_dz = transition_depth.value().max(0.0) * 0.5;
    (
        Length::new((z_f - half_dz).max(0.0)),
        Length::new(z_f + half_dz),
    )
}

pub fn liquid_condensate_density(total_condensate: Density, ice_fraction: f64) -> Density {
    let frac = ice_fraction.clamp(0.0, 1.0);
    Density::new(total_condensate.value() * (1.0 - frac))
}

pub fn ice_condensate_density(total_condensate: Density, ice_fraction: f64) -> Density {
    let frac = ice_fraction.clamp(0.0, 1.0);
    Density::new(total_condensate.value() * frac)
}