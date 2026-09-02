use crate::units::constants::SECONDS_PER_YEAR;
use crate::units::{ Speed, Temperature };
use crate::units::constants::{
    MIAMI_MODEL_ASYMPTOTIC_MAX_NPP,
    MIAMI_MODEL_TEMP_A,
    MIAMI_MODEL_TEMP_B,
    MIAMI_MODEL_PRECIP_K,
};
use serde::{ Deserialize, Serialize };

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PrimaryProductivityConfidence {
    HighAqueousBiochemistry,
    LowNonAqueousSpeculative,
}

impl PrimaryProductivityConfidence {
    pub fn is_high_confidence(&self) -> bool {
        matches!(self, Self::HighAqueousBiochemistry)
    }

    pub fn is_speculative(&self) -> bool {
        matches!(self, Self::LowNonAqueousSpeculative)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StandardPrimaryHabitability {
    pub npp_temperature: f64,
    pub npp_precipitation: f64,
    pub npp_final: f64,
    pub sph_index: f64,
    pub confidence: PrimaryProductivityConfidence,
}

impl StandardPrimaryHabitability {
    pub fn new(
        npp_temperature: f64,
        npp_precipitation: f64,
        npp_final: f64,
        sph_index: f64,
        confidence: PrimaryProductivityConfidence
    ) -> Self {
        Self {
            npp_temperature,
            npp_precipitation,
            npp_final,
            sph_index,
            confidence,
        }
    }

    pub fn npp_temperature(&self) -> f64 {
        self.npp_temperature
    }

    pub fn npp_precipitation(&self) -> f64 {
        self.npp_precipitation
    }

    pub fn npp_final(&self) -> f64 {
        self.npp_final
    }

    pub fn sph_index(&self) -> f64 {
        self.sph_index
    }

    pub fn confidence(&self) -> PrimaryProductivityConfidence {
        self.confidence
    }
}

pub fn annual_precipitation_mm_from_rate(precipitation_rate: Speed) -> f64 {
    let rate_m_per_s = precipitation_rate.value();
    if !rate_m_per_s.is_finite() || rate_m_per_s <= 0.0 {
        return 0.0;
    }
    rate_m_per_s * 1000.0 * SECONDS_PER_YEAR
}

pub fn npp_temperature_limited(temperature: Temperature) -> f64 {
    let t_kelvin = temperature.value();
    if !t_kelvin.is_finite() || t_kelvin <= 0.0 {
        return 0.0;
    }

    let t_celsius = t_kelvin - 273.15;
    let exponent = MIAMI_MODEL_TEMP_A - MIAMI_MODEL_TEMP_B * t_celsius;

    if exponent >= 50.0 {
        0.0
    } else if exponent <= -50.0 {
        MIAMI_MODEL_ASYMPTOTIC_MAX_NPP
    } else {
        let denom = 1.0 + exponent.exp();
        if !denom.is_finite() || denom <= 0.0 {
            0.0
        } else {
            (MIAMI_MODEL_ASYMPTOTIC_MAX_NPP / denom).clamp(0.0, MIAMI_MODEL_ASYMPTOTIC_MAX_NPP)
        }
    }
}

pub fn npp_precipitation_limited(annual_precipitation_mm: f64) -> f64 {
    if !annual_precipitation_mm.is_finite() || annual_precipitation_mm <= 0.0 {
        return 0.0;
    }

    let exponent = -MIAMI_MODEL_PRECIP_K * annual_precipitation_mm;
    if exponent <= -50.0 {
        MIAMI_MODEL_ASYMPTOTIC_MAX_NPP
    } else {
        let factor = 1.0 - exponent.exp();
        (MIAMI_MODEL_ASYMPTOTIC_MAX_NPP * factor).clamp(0.0, MIAMI_MODEL_ASYMPTOTIC_MAX_NPP)
    }
}

pub fn miami_model_npp(temperature: Temperature, annual_precipitation_mm: f64) -> f64 {
    let npp_t = npp_temperature_limited(temperature);
    let npp_p = npp_precipitation_limited(annual_precipitation_mm);
    npp_t.min(npp_p)
}

pub fn sph_index(temperature: Temperature, annual_precipitation_mm: f64) -> f64 {
    let npp = miami_model_npp(temperature, annual_precipitation_mm);
    (npp / MIAMI_MODEL_ASYMPTOTIC_MAX_NPP).clamp(0.0, 1.0)
}

pub fn standard_primary_habitability(
    temperature: Temperature,
    annual_precipitation_mm: f64,
    is_aqueous_solvent: bool
) -> StandardPrimaryHabitability {
    let npp_t = npp_temperature_limited(temperature);
    let npp_p = npp_precipitation_limited(annual_precipitation_mm);
    let npp_final = npp_t.min(npp_p);
    let sph = (npp_final / MIAMI_MODEL_ASYMPTOTIC_MAX_NPP).clamp(0.0, 1.0);
    let confidence = if is_aqueous_solvent {
        PrimaryProductivityConfidence::HighAqueousBiochemistry
    } else {
        PrimaryProductivityConfidence::LowNonAqueousSpeculative
    };

    StandardPrimaryHabitability::new(npp_t, npp_p, npp_final, sph, confidence)
}

pub fn standard_primary_habitability_from_rate(
    temperature: Temperature,
    precipitation_rate: Speed,
    is_aqueous_solvent: bool
) -> StandardPrimaryHabitability {
    let precip_mm = annual_precipitation_mm_from_rate(precipitation_rate);
    standard_primary_habitability(temperature, precip_mm, is_aqueous_solvent)
}
