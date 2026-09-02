use astronomicon_core::units::{Irradiance, Luminosity, Vector3};

pub fn solar_incidence_factor(
    panel_normal_world: Vector3,
    sun_direction_world: Vector3,
    is_sun_tracking: bool,
) -> f64 {
    if is_sun_tracking {
        return 1.0;
    }

    let n = panel_normal_world.normalized();
    let s = sun_direction_world.normalized();
    let dot = n.dot(&s);

    if !dot.is_finite() || dot <= 0.0 {
        0.0
    } else {
        dot.min(1.0)
    }
}

pub fn solar_panel_incident_power(
    irradiance: Irradiance,
    area_m2: f64,
    incidence_factor: f64,
) -> Luminosity {
    let irr = irradiance.value();
    let a = area_m2;
    let inc = incidence_factor.clamp(0.0, 1.0);

    if irr <= 0.0
        || a <= 0.0
        || inc <= 0.0
        || !irr.is_finite()
        || !a.is_finite()
        || !inc.is_finite()
    {
        return Luminosity::new(0.0);
    }

    Luminosity::new(irr * a * inc)
}

pub fn solar_panel_electrical_output(
    irradiance: Irradiance,
    area_m2: f64,
    conversion_efficiency: f64,
    solar_incidence_factor: f64,
    is_eclipsed: bool,
) -> Luminosity {
    if is_eclipsed {
        return Luminosity::new(0.0);
    }

    let p_inc = solar_panel_incident_power(irradiance, area_m2, solar_incidence_factor).value();
    let eff = conversion_efficiency.clamp(0.0, 1.0);

    if p_inc <= 0.0 || !p_inc.is_finite() || !eff.is_finite() || eff <= 0.0 {
        Luminosity::new(0.0)
    } else {
        Luminosity::new(p_inc * eff)
    }
}

pub fn solar_panel_waste_heat(
    irradiance: Irradiance,
    area_m2: f64,
    absorptivity: f64,
    conversion_efficiency: f64,
    solar_incidence_factor: f64,
    is_eclipsed: bool,
) -> Luminosity {
    if is_eclipsed {
        return Luminosity::new(0.0);
    }

    let p_inc = solar_panel_incident_power(irradiance, area_m2, solar_incidence_factor).value();
    let alpha = absorptivity.clamp(0.0, 1.0);
    let eff = conversion_efficiency.clamp(0.0, 1.0);

    if p_inc <= 0.0 || !p_inc.is_finite() {
        return Luminosity::new(0.0);
    }

    let p_abs = p_inc * alpha;
    let p_elec = p_inc * eff;
    let diff = (p_abs - p_elec).max(0.0);

    if !diff.is_finite() {
        Luminosity::new(0.0)
    } else {
        Luminosity::new(diff)
    }
}