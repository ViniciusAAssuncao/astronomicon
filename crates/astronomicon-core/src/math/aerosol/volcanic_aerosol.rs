use crate::math::volcanism::VolcanicEruptionStyle;
use crate::units::{Density, Duration, Length, MassRate};
use std::f64::consts::PI;

pub fn volcanic_sulfate_aerosol_density(
    so2_mass_rate: MassRate,
    planet_radius: Length,
    scale_height: Length,
    residence_time: Duration,
) -> Density {
    let q_so2 = so2_mass_rate.value();
    let r = planet_radius.value();
    let h = scale_height.value();
    let tau = residence_time.value();

    if q_so2 <= 0.0
        || r <= 0.0
        || h <= 0.0
        || tau <= 0.0
        || !q_so2.is_finite()
        || !r.is_finite()
        || !h.is_finite()
        || !tau.is_finite()
    {
        return Density::new(0.0);
    }

    let volume_atm = 4.0 * PI * r * r * h;
    if volume_atm <= 0.0 || !volume_atm.is_finite() {
        return Density::new(0.0);
    }

    let conversion_factor = 1.53;
    let sulfate_mass = q_so2 * conversion_factor * tau;
    let rho = (sulfate_mass / volume_atm).clamp(0.0, 0.01);

    if !rho.is_finite() {
        Density::new(0.0)
    } else {
        Density::new(rho)
    }
}

pub fn volcanic_ash_aerosol_density(
    eruption_style: VolcanicEruptionStyle,
    magma_extrusion_rate: MassRate,
    planet_radius: Length,
    scale_height: Length,
    residence_time: Duration,
) -> Density {
    let q_magma = magma_extrusion_rate.value();
    let r = planet_radius.value();
    let h = scale_height.value();
    let tau = residence_time.value();

    if q_magma <= 0.0
        || r <= 0.0
        || h <= 0.0
        || tau <= 0.0
        || !q_magma.is_finite()
        || !r.is_finite()
        || !h.is_finite()
        || !tau.is_finite()
    {
        return Density::new(0.0);
    }

    let ash_fraction = match eruption_style {
        VolcanicEruptionStyle::Explosive => 0.15,
        VolcanicEruptionStyle::Effusive => 0.001,
        VolcanicEruptionStyle::Cryovolcanic => 0.02,
        VolcanicEruptionStyle::SubaqueousEffusive | VolcanicEruptionStyle::Inactive => 0.0,
    };

    if ash_fraction <= 0.0 {
        return Density::new(0.0);
    }

    let volume_atm = 4.0 * PI * r * r * h;
    if volume_atm <= 0.0 || !volume_atm.is_finite() {
        return Density::new(0.0);
    }

    let ash_mass = q_magma * ash_fraction * tau;
    let rho = (ash_mass / volume_atm).clamp(0.0, 0.05);

    if !rho.is_finite() {
        Density::new(0.0)
    } else {
        Density::new(rho)
    }
}

pub fn volcanic_aerosol_density(
    so2_mass_rate: MassRate,
    eruption_style: VolcanicEruptionStyle,
    magma_extrusion_rate: MassRate,
    planet_radius: Length,
    scale_height: Length,
) -> Density {
    let tau_sulfate = Duration::new(2.592e6);
    let tau_ash = Duration::new(6.048e5);

    let rho_sulfate =
        volcanic_sulfate_aerosol_density(so2_mass_rate, planet_radius, scale_height, tau_sulfate);
    let rho_ash = volcanic_ash_aerosol_density(
        eruption_style,
        magma_extrusion_rate,
        planet_radius,
        scale_height,
        tau_ash,
    );

    Density::new(rho_sulfate.value() + rho_ash.value())
}