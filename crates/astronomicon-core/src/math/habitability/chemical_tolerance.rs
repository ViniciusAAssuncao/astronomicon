use crate::math::precipitation::CondensatePrimaryClass;
use crate::units::Temperature;
use crate::units::constants::{
    KNOWN_LIFE_MIN_TEMPERATURE_K,
    KNOWN_LIFE_MAX_TEMPERATURE_K,
    TEMPERATURE_VIABILITY_TRANSITION_WIDTH_K,
    KNOWN_LIFE_MIN_PH,
    KNOWN_LIFE_MAX_PH,
    PH_VIABILITY_TRANSITION_WIDTH,
    CRYOGENIC_HYDROCARBON_MIN_TEMPERATURE_K,
    CRYOGENIC_HYDROCARBON_MAX_TEMPERATURE_K,
    CRYOGENIC_HYDROCARBON_TRANSITION_WIDTH_K,
};
use serde::{ Deserialize, Serialize };

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChemicalToleranceConfidence {
    HighKnownAqueousBiochemistry,
    LowSpeculativeCryogenicHydrocarbon,
    InviableOrUnknownBiochemistry,
}

impl ChemicalToleranceConfidence {
    pub fn is_high_confidence(&self) -> bool {
        matches!(self, Self::HighKnownAqueousBiochemistry)
    }

    pub fn is_speculative(&self) -> bool {
        matches!(self, Self::LowSpeculativeCryogenicHydrocarbon)
    }

    pub fn is_inviable_or_unknown(&self) -> bool {
        matches!(self, Self::InviableOrUnknownBiochemistry)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ChemicalToleranceAssessment {
    pub temperature_viability: f64,
    pub ph_viability: Option<f64>,
    pub overall_viability: f64,
    pub confidence: ChemicalToleranceConfidence,
}

impl ChemicalToleranceAssessment {
    pub fn new(
        temperature_viability: f64,
        ph_viability: Option<f64>,
        overall_viability: f64,
        confidence: ChemicalToleranceConfidence
    ) -> Self {
        Self {
            temperature_viability,
            ph_viability,
            overall_viability,
            confidence,
        }
    }

    pub fn temperature_viability(&self) -> f64 {
        self.temperature_viability
    }

    pub fn ph_viability(&self) -> Option<f64> {
        self.ph_viability
    }

    pub fn overall_viability(&self) -> f64 {
        self.overall_viability
    }

    pub fn confidence(&self) -> ChemicalToleranceConfidence {
        self.confidence
    }
}

pub fn logistic_smooth_window(
    value: f64,
    lower_bound: f64,
    upper_bound: f64,
    transition_width: f64
) -> f64 {
    if
        !value.is_finite() ||
        !lower_bound.is_finite() ||
        !upper_bound.is_finite() ||
        lower_bound >= upper_bound
    {
        return 0.0;
    }

    let width = if transition_width.is_finite() && transition_width > 0.0 {
        transition_width
    } else {
        1.0
    };

    let scale = width * 0.25;

    let arg_low = (value - lower_bound) / scale;
    let s_low = if arg_low <= -50.0 {
        0.0
    } else if arg_low >= 50.0 {
        1.0
    } else {
        1.0 / (1.0 + (-arg_low).exp())
    };

    let arg_high = (value - upper_bound) / scale;
    let s_high = if arg_high >= 50.0 {
        0.0
    } else if arg_high <= -50.0 {
        1.0
    } else {
        1.0 / (1.0 + arg_high.exp())
    };

    (s_low * s_high).clamp(0.0, 1.0)
}

pub fn temperature_viability(
    temperature: Temperature,
    lower_limit: Temperature,
    upper_limit: Temperature,
    transition_width: f64
) -> f64 {
    let t = temperature.value();
    let t_min = lower_limit.value();
    let t_max = upper_limit.value();
    if t <= 0.0 || !t.is_finite() {
        return 0.0;
    }
    logistic_smooth_window(t, t_min, t_max, transition_width)
}

pub fn aqueous_temperature_viability(temperature: Temperature) -> f64 {
    temperature_viability(
        temperature,
        Temperature::new(KNOWN_LIFE_MIN_TEMPERATURE_K),
        Temperature::new(KNOWN_LIFE_MAX_TEMPERATURE_K),
        TEMPERATURE_VIABILITY_TRANSITION_WIDTH_K
    )
}

pub fn ph_viability(ph: f64, lower_limit: f64, upper_limit: f64, transition_width: f64) -> f64 {
    if !ph.is_finite() {
        return 0.0;
    }
    logistic_smooth_window(ph, lower_limit, upper_limit, transition_width)
}

pub fn aqueous_ph_viability(ph: f64) -> f64 {
    ph_viability(ph, KNOWN_LIFE_MIN_PH, KNOWN_LIFE_MAX_PH, PH_VIABILITY_TRANSITION_WIDTH)
}

pub fn cryogenic_hydrocarbon_temperature_viability(temperature: Temperature) -> f64 {
    temperature_viability(
        temperature,
        Temperature::new(CRYOGENIC_HYDROCARBON_MIN_TEMPERATURE_K),
        Temperature::new(CRYOGENIC_HYDROCARBON_MAX_TEMPERATURE_K),
        CRYOGENIC_HYDROCARBON_TRANSITION_WIDTH_K
    )
}

pub fn evaluate_chemical_tolerance(
    condensate_class: CondensatePrimaryClass,
    temperature: Temperature,
    ph: Option<f64>
) -> ChemicalToleranceAssessment {
    match condensate_class {
        CondensatePrimaryClass::AqueousMolecular => {
            let t_viab = aqueous_temperature_viability(temperature);
            let ph_val = ph.unwrap_or(7.0);
            let ph_viab = aqueous_ph_viability(ph_val);
            let overall = (t_viab * ph_viab).clamp(0.0, 1.0);
            ChemicalToleranceAssessment::new(
                t_viab,
                Some(ph_viab),
                overall,
                ChemicalToleranceConfidence::HighKnownAqueousBiochemistry
            )
        }
        CondensatePrimaryClass::CryogenicHydrocarbon => {
            let t_viab = cryogenic_hydrocarbon_temperature_viability(temperature);
            ChemicalToleranceAssessment::new(
                t_viab,
                None,
                t_viab,
                ChemicalToleranceConfidence::LowSpeculativeCryogenicHydrocarbon
            )
        }
        CondensatePrimaryClass::StrongAcid => {
            let t_viab = aqueous_temperature_viability(temperature);
            let ph_viab = ph.map(aqueous_ph_viability);
            ChemicalToleranceAssessment::new(
                t_viab,
                ph_viab,
                0.0,
                ChemicalToleranceConfidence::InviableOrUnknownBiochemistry
            )
        }
        CondensatePrimaryClass::OtherCovalent => {
            let t_viab = aqueous_temperature_viability(temperature);
            ChemicalToleranceAssessment::new(
                t_viab,
                None,
                0.0,
                ChemicalToleranceConfidence::InviableOrUnknownBiochemistry
            )
        }
    }
}
