use astronomicon_core::units::Duration;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RadioisotopeType {
    Plutonium238,
    Americium241,
    Strontium90,
    Polonium210,
    Curium244,
    Curium242,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RadioisotopeProperties {
    pub half_life: Duration,
    pub specific_thermal_power_bol_w_per_kg: f64,
}

impl RadioisotopeProperties {
    pub const fn new(half_life: Duration, specific_thermal_power_bol_w_per_kg: f64) -> Self {
        Self {
            half_life,
            specific_thermal_power_bol_w_per_kg,
        }
    }

    pub fn half_life(&self) -> Duration {
        self.half_life
    }

    pub fn specific_thermal_power_bol_w_per_kg(&self) -> f64 {
        self.specific_thermal_power_bol_w_per_kg
    }
}

pub fn radioisotope_properties_of(isotope: RadioisotopeType) -> RadioisotopeProperties {
    match isotope {
        RadioisotopeType::Plutonium238 => {
            RadioisotopeProperties::new(Duration::new(2_768_863_824.0), 567.0)
        }
        RadioisotopeType::Americium241 => {
            RadioisotopeProperties::new(Duration::new(13_639_194_720.0), 114.0)
        }
        RadioisotopeType::Strontium90 => {
            RadioisotopeProperties::new(Duration::new(908_543_304.0), 926.0)
        }
        RadioisotopeType::Polonium210 => {
            RadioisotopeProperties::new(Duration::new(11_955_686.4), 141_300.0)
        }
        RadioisotopeType::Curium244 => {
            RadioisotopeProperties::new(Duration::new(571_192_560.0), 2_800.0)
        }
        RadioisotopeType::Curium242 => {
            RadioisotopeProperties::new(Duration::new(14_065_920.0), 120_000.0)
        }
    }
}