use crate::constants::constants::SYNCHRONOUS_ROTATION_TOLERANCE;
use crate::domain::rotation_classification::RotationClassification;
use crate::math::day_length::solar_day_length;
use astronomicon_core::domain::{OrbitalElements, Planet};
use astronomicon_core::math::kepler::orbital_period;
use astronomicon_core::units::{Angle, Duration, GravitationalParameter};
use serde::{Deserialize, Serialize};
use std::f64::consts::FRAC_PI_2;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlanetaryDayInfo {
    pub sidereal_day: Option<Duration>,
    pub solar_day: Option<Duration>,
    pub classification: RotationClassification,
    pub orbital_period: Option<Duration>,
}

impl PlanetaryDayInfo {
    pub fn new(
        sidereal_day: Option<Duration>,
        solar_day: Option<Duration>,
        classification: RotationClassification,
        orbital_period: Option<Duration>,
    ) -> Self {
        Self {
            sidereal_day,
            solar_day,
            classification,
            orbital_period,
        }
    }

    pub fn sidereal_day(&self) -> Option<Duration> {
        self.sidereal_day
    }

    pub fn solar_day(&self) -> Option<Duration> {
        self.solar_day
    }

    pub fn classification(&self) -> RotationClassification {
        self.classification
    }

    pub fn orbital_period(&self) -> Option<Duration> {
        self.orbital_period
    }
}

pub fn is_retrograde_obliquity(obliquity: Option<Angle>) -> bool {
    match obliquity {
        Some(angle) => {
            let val = angle.value();
            val > FRAC_PI_2 && val.is_finite()
        }
        None => false,
    }
}

pub fn is_tidally_synchronized(
    rotation_period: Duration,
    orbital_period: Duration,
    tolerance: f64,
) -> bool {
    let t_rot = rotation_period.value();
    let t_orb = orbital_period.value();

    if !t_rot.is_finite() || !t_orb.is_finite() || t_rot <= 0.0 || t_orb <= 0.0 {
        return false;
    }

    (t_rot - t_orb).abs() / t_orb <= tolerance
}

pub fn analyze_planetary_day(
    planet: &Planet,
    central_body_mu: GravitationalParameter,
    orbital_elements: &OrbitalElements,
) -> PlanetaryDayInfo {
    let t_orb = orbital_period(orbital_elements.semi_major_axis(), central_body_mu);

    let t_rot = match planet.rotation_period() {
        Some(rot) if rot.value() > 0.0 && rot.value().is_finite() => rot,
        _ => {
            return PlanetaryDayInfo::new(
                None,
                None,
                RotationClassification::NoRotationalData,
                t_orb,
            );
        }
    };

    let is_retrograde = is_retrograde_obliquity(planet.obliquity());

    if let Some(orb) = t_orb {
        if !is_retrograde && is_tidally_synchronized(t_rot, orb, SYNCHRONOUS_ROTATION_TOLERANCE) {
            return PlanetaryDayInfo::new(
                Some(t_rot),
                None,
                RotationClassification::Synchronous,
                Some(orb),
            );
        }
    }

    let classification = if is_retrograde {
        RotationClassification::Retrograde
    } else {
        RotationClassification::Prograde
    };

    let t_solar = match t_orb {
        Some(orb) => solar_day_length(t_rot, orb, is_retrograde),
        None => None,
    };

    PlanetaryDayInfo::new(Some(t_rot), t_solar, classification, t_orb)
}