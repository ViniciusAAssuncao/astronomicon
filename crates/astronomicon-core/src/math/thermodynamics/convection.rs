use crate::chemistry::solvent::SolventProperties;
use crate::math::thermodynamics::lapse_rate::{
    dry_adiabatic_lapse_rate, moist_adiabatic_lapse_rate,
};
use crate::units::constants::UNIVERSAL_GAS_CONSTANT;
use crate::units::{
    Acceleration, Length, MolarMass, Pressure, SpecificEnergy, Temperature,
    TemperatureGradient,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ParcelBuoyancyStep {
    pub altitude: Length,
    pub parcel_temperature: Temperature,
    pub environmental_temperature: Temperature,
    pub buoyancy: f64,
}

pub fn lifting_condensation_level(
    surface_temperature: Temperature,
    dew_point: Temperature,
    dry_lapse_rate: TemperatureGradient,
    scale_height: Length,
    enthalpy_of_vaporization: f64,
) -> Length {
    let ts = surface_temperature.value();
    let td = dew_point.value();
    let gamma_d = dry_lapse_rate.value();
    let h = scale_height.value();
    let delta_h = enthalpy_of_vaporization;

    if ts <= 0.0 || td <= 0.0 || !ts.is_finite() || !td.is_finite() {
        return Length::new(0.0);
    }

    if td >= ts {
        return Length::new(0.0);
    }

    if gamma_d <= 0.0 || !gamma_d.is_finite() {
        return Length::new(0.0);
    }

    if h <= 0.0 || delta_h <= 0.0 || !h.is_finite() || !delta_h.is_finite() {
        let z = (ts - td) / gamma_d;
        return Length::new(z.max(0.0));
    }

    let a = gamma_d * (UNIVERSAL_GAS_CONSTANT / (delta_h * h));
    let b = gamma_d / td - (ts * UNIVERSAL_GAS_CONSTANT) / (delta_h * h);
    let c = 1.0 - ts / td;

    if a.abs() < 1e-18 {
        if b.abs() > 1e-18 {
            let z = -c / b;
            Length::new(z.max(0.0))
        } else {
            let z = (ts - td) / gamma_d;
            Length::new(z.max(0.0))
        }
    } else {
        let disc = b * b - 4.0 * a * c;
        if disc < 0.0 {
            let z = (ts - td) / gamma_d;
            Length::new(z.max(0.0))
        } else {
            let z = (-b + disc.sqrt()) / (2.0 * a);
            Length::new(z.max(0.0))
        }
    }
}

pub fn integrate_parcel_buoyancy_profile(
    surface_temperature: Temperature,
    surface_pressure: Pressure,
    dew_point: Temperature,
    environmental_lapse_rate: TemperatureGradient,
    scale_height: Length,
    gravity: Acceleration,
    specific_heat_capacity: f64,
    tropopause_altitude: Length,
    tropopause_temperature: Temperature,
    solvent_properties: &SolventProperties,
    solvent_molar_mass: MolarMass,
    atmospheric_molar_mass: MolarMass,
) -> Vec<ParcelBuoyancyStep> {
    let ts = surface_temperature.value();
    let ps = surface_pressure.value();
    let td = dew_point.value();
    let h = scale_height.value();
    let g = gravity.value();
    let cp = specific_heat_capacity;
    let gamma_env = environmental_lapse_rate.value();
    let z_tropo = tropopause_altitude.value();
    let t_tropo = tropopause_temperature.value();

    if ts <= 0.0
        || ps <= 0.0
        || td <= 0.0
        || h <= 0.0
        || g <= 0.0
        || cp <= 0.0
        || !ts.is_finite()
        || !ps.is_finite()
        || !td.is_finite()
        || !h.is_finite()
        || !g.is_finite()
        || !cp.is_finite()
    {
        return Vec::new();
    }

    let gamma_d = dry_adiabatic_lapse_rate(gravity, specific_heat_capacity);
    let z_lcl = lifting_condensation_level(
        surface_temperature,
        dew_point,
        gamma_d,
        scale_height,
        solvent_properties.enthalpy_of_vaporization,
    )
    .value();

    let z_top = if z_tropo.is_finite() && z_tropo > 0.0 {
        z_tropo
    } else {
        (z_lcl * 2.0).max(10.0 * h)
    };

    let dz = (z_top / 500.0).clamp(10.0, 100.0);
    let steps = ((z_top / dz).ceil() as usize).max(10);

    let mut profile = Vec::with_capacity(steps + 1);

    let mut t_p = ts;
    let mut prev_z = 0.0;

    profile.push(ParcelBuoyancyStep {
        altitude: Length::new(0.0),
        parcel_temperature: Temperature::new(ts),
        environmental_temperature: Temperature::new(ts),
        buoyancy: 0.0,
    });

    for i in 1..=steps {
        let mut z = (i as f64) * dz;
        if z > z_top {
            z = z_top;
        }

        let t_env = if z >= z_tropo && z_tropo.is_finite() && z_tropo > 0.0 {
            t_tropo
        } else {
            (ts - gamma_env * z).max(t_tropo)
        };

        if z <= z_lcl {
            t_p = (ts - gamma_d.value() * z).max(0.0);
        } else if prev_z < z_lcl {
            t_p = (ts - gamma_d.value() * z_lcl).max(0.0);
            let p_lcl = ps * (-z_lcl / h).exp();
            let gamma_m = moist_adiabatic_lapse_rate(
                gravity,
                cp,
                Temperature::new(t_p),
                Pressure::new(p_lcl),
                solvent_properties,
                solvent_molar_mass,
                atmospheric_molar_mass,
            )
            .value();
            t_p = (t_p - gamma_m * (z - z_lcl)).max(0.0);
        } else {
            let p_prev = ps * (-prev_z / h).exp();
            let gamma_m = moist_adiabatic_lapse_rate(
                gravity,
                cp,
                Temperature::new(t_p),
                Pressure::new(p_prev),
                solvent_properties,
                solvent_molar_mass,
                atmospheric_molar_mass,
            )
            .value();
            t_p = (t_p - gamma_m * (z - prev_z)).max(0.0);
        }

        let b = if t_env > 0.0 {
            (g * (t_p - t_env)) / t_env
        } else {
            0.0
        };

        profile.push(ParcelBuoyancyStep {
            altitude: Length::new(z),
            parcel_temperature: Temperature::new(t_p),
            environmental_temperature: Temperature::new(t_env),
            buoyancy: b,
        });

        prev_z = z;
        if z >= z_top {
            break;
        }
    }

    profile
}

pub fn level_of_free_convection(
    profile: &[ParcelBuoyancyStep],
    lcl_altitude: Length,
) -> Option<Length> {
    let z_lcl = lcl_altitude.value();
    let mut found_lfc = None;

    for i in 0..profile.len() {
        let step = &profile[i];
        if step.altitude.value() < z_lcl - 1e-4 {
            continue;
        }
        if step.parcel_temperature.value() > step.environmental_temperature.value() + 1e-4 {
            if i > 0 && profile[i - 1].altitude.value() >= z_lcl - 1e-4 {
                let prev = &profile[i - 1];
                let dt_prev =
                    prev.parcel_temperature.value() - prev.environmental_temperature.value();
                let dt_curr =
                    step.parcel_temperature.value() - step.environmental_temperature.value();
                if dt_prev <= 0.0 && dt_curr > 0.0 {
                    let frac = -dt_prev / (dt_curr - dt_prev);
                    let z_interp =
                        prev.altitude.value() + frac * (step.altitude.value() - prev.altitude.value());
                    found_lfc = Some(Length::new(z_interp));
                    break;
                }
            }
            found_lfc = Some(step.altitude);
            break;
        }
    }

    found_lfc
}

pub fn equilibrium_level(profile: &[ParcelBuoyancyStep], lfc_altitude: Length) -> Option<Length> {
    let z_lfc = lfc_altitude.value();
    let mut past_lfc = false;

    for i in 0..profile.len() {
        let step = &profile[i];
        if step.altitude.value() < z_lfc {
            continue;
        }
        if step.parcel_temperature.value() >= step.environmental_temperature.value() {
            past_lfc = true;
        } else if past_lfc {
            if i > 0 {
                let prev = &profile[i - 1];
                let dt_prev =
                    prev.parcel_temperature.value() - prev.environmental_temperature.value();
                let dt_curr =
                    step.parcel_temperature.value() - step.environmental_temperature.value();
                if dt_prev > 0.0 && dt_curr <= 0.0 {
                    let frac = dt_prev / (dt_prev - dt_curr);
                    let z_interp =
                        prev.altitude.value() + frac * (step.altitude.value() - prev.altitude.value());
                    return Some(Length::new(z_interp));
                }
            }
            return Some(step.altitude);
        }
    }

    if past_lfc {
        profile.last().map(|s| s.altitude)
    } else {
        None
    }
}

pub fn convective_available_potential_energy(
    profile: &[ParcelBuoyancyStep],
    lfc_altitude: Length,
    equilibrium_level: Length,
) -> SpecificEnergy {
    let z_lfc = lfc_altitude.value();
    let z_el = equilibrium_level.value();

    if z_el <= z_lfc || profile.is_empty() {
        return SpecificEnergy::new(0.0);
    }

    let mut cape = 0.0;

    for i in 0..profile.len().saturating_sub(1) {
        let s0 = &profile[i];
        let s1 = &profile[i + 1];

        let z0 = s0.altitude.value();
        let z1 = s1.altitude.value();

        if z1 <= z_lfc || z0 >= z_el {
            continue;
        }

        let seg_start = z0.max(z_lfc);
        let seg_end = z1.min(z_el);
        let dz = seg_end - seg_start;

        if dz <= 0.0 {
            continue;
        }

        let b0 = s0.buoyancy.max(0.0);
        let b1 = s1.buoyancy.max(0.0);

        cape += 0.5 * (b0 + b1) * dz;
    }

    if !cape.is_finite() || cape <= 0.0 {
        SpecificEnergy::new(0.0)
    } else {
        SpecificEnergy::new(cape)
    }
}

pub fn convective_inhibition(
    profile: &[ParcelBuoyancyStep],
    lfc_altitude: Length,
) -> SpecificEnergy {
    let z_lfc = lfc_altitude.value();

    if z_lfc <= 0.0 || profile.is_empty() {
        return SpecificEnergy::new(0.0);
    }

    let mut cin = 0.0;

    for i in 0..profile.len().saturating_sub(1) {
        let s0 = &profile[i];
        let s1 = &profile[i + 1];

        let z0 = s0.altitude.value();
        let z1 = s1.altitude.value();

        if z0 >= z_lfc {
            break;
        }

        let seg_end = z1.min(z_lfc);
        let dz = seg_end - z0;

        if dz <= 0.0 {
            continue;
        }

        let b0_neg = (-s0.buoyancy).max(0.0);
        let b1_neg = (-s1.buoyancy).max(0.0);

        cin += 0.5 * (b0_neg + b1_neg) * dz;
    }

    if !cin.is_finite() || cin <= 0.0 {
        SpecificEnergy::new(0.0)
    } else {
        SpecificEnergy::new(cin)
    }
}

pub fn cloud_top_altitude(
    lcl_altitude: Length,
    surface_temperature: Temperature,
    surface_pressure: Pressure,
    environmental_lapse_rate: TemperatureGradient,
    scale_height: Length,
    gravity: Acceleration,
    specific_heat_capacity: f64,
    tropopause_altitude: Length,
    tropopause_temperature: Temperature,
    solvent_properties: &SolventProperties,
    solvent_molar_mass: MolarMass,
    atmospheric_molar_mass: MolarMass,
) -> Length {
    let z_lcl = lcl_altitude.value();
    let ts = surface_temperature.value();
    let ps = surface_pressure.value();
    let gamma_env = environmental_lapse_rate.value();
    let h = scale_height.value();
    let z_tropo = tropopause_altitude.value();
    let t_tropo = tropopause_temperature.value();
    let cp = specific_heat_capacity;

    if z_lcl < 0.0 || !z_lcl.is_finite() || ts <= 0.0 || ps <= 0.0 || h <= 0.0 {
        return Length::new(0.0);
    }

    if z_tropo.is_finite() && z_tropo > 0.0 && z_lcl >= z_tropo {
        return lcl_altitude;
    }

    let gamma_d = dry_adiabatic_lapse_rate(gravity, specific_heat_capacity);
    let mut t_p = (ts - gamma_d.value() * z_lcl).max(0.0);
    let t_env_lcl = if z_tropo.is_finite() && z_tropo > 0.0 && z_lcl >= z_tropo {
        t_tropo
    } else {
        (ts - gamma_env * z_lcl).max(t_tropo)
    };

    if t_p <= t_env_lcl {
        return lcl_altitude;
    }

    let z_top = if z_tropo.is_finite() && z_tropo > 0.0 {
        z_tropo
    } else {
        (z_lcl * 2.0).max(10.0 * h)
    };

    let dz = ((z_top - z_lcl) / 200.0).clamp(10.0, 100.0);
    let mut z = z_lcl;

    while z < z_top {
        z += dz;
        if z > z_top {
            z = z_top;
        }

        let p_z = ps * (-z / h).exp();
        let gamma_m = moist_adiabatic_lapse_rate(
            gravity,
            cp,
            Temperature::new(t_p),
            Pressure::new(p_z),
            solvent_properties,
            solvent_molar_mass,
            atmospheric_molar_mass,
        )
        .value();

        t_p = (t_p - gamma_m * dz).max(0.0);
        let t_env = if z >= z_tropo && z_tropo.is_finite() && z_tropo > 0.0 {
            t_tropo
        } else {
            (ts - gamma_env * z).max(t_tropo)
        };

        if t_p <= t_env {
            return Length::new(z);
        }

        if z >= z_top {
            return Length::new(z_top);
        }
    }

    Length::new(z.max(z_lcl))
}