use crate::units::constants::SECONDS_PER_YEAR;
use crate::units::{Duration, RadiationDose};
use serde::{Deserialize, Serialize};

pub const D37_ACUTE_COMPLEX_LIFE_SV: f64 = 4.0;
pub const D37_ACUTE_EXTREMOPHILE_SV: f64 = 10000.0;
pub const D37_ANNUAL_CHRONIC_COMPLEX_LIFE_SV: f64 = 1.0;
pub const D37_ANNUAL_CHRONIC_EXTREMOPHILE_SV: f64 = 50000.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RadiobiologicalTarget {
    ComplexEukaryote,
    RadioresistantExtremophile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RadiationExposureRegime {
    Acute,
    AnnualChronic,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RadiationToleranceAssessment {
    pub annual_dose: RadiationDose,
    pub complex_life_survival_fraction: f64,
    pub extremophile_survival_fraction: f64,
    pub is_complex_life_viable: bool,
    pub is_extremophile_viable: bool,
}

impl RadiationToleranceAssessment {
    pub fn new(
        annual_dose: RadiationDose,
        complex_life_survival_fraction: f64,
        extremophile_survival_fraction: f64,
        is_complex_life_viable: bool,
        is_extremophile_viable: bool,
    ) -> Self {
        Self {
            annual_dose,
            complex_life_survival_fraction,
            extremophile_survival_fraction,
            is_complex_life_viable,
            is_extremophile_viable,
        }
    }

    pub fn annual_dose(&self) -> RadiationDose {
        self.annual_dose
    }

    pub fn complex_life_survival_fraction(&self) -> f64 {
        self.complex_life_survival_fraction
    }

    pub fn extremophile_survival_fraction(&self) -> f64 {
        self.extremophile_survival_fraction
    }

    pub fn is_complex_life_viable(&self) -> bool {
        self.is_complex_life_viable
    }

    pub fn is_extremophile_viable(&self) -> bool {
        self.is_extremophile_viable
    }
}

pub fn d37_reference_dose(
    target: RadiobiologicalTarget,
    regime: RadiationExposureRegime,
) -> RadiationDose {
    match (target, regime) {
        (RadiobiologicalTarget::ComplexEukaryote, RadiationExposureRegime::Acute) => {
            RadiationDose::new(D37_ACUTE_COMPLEX_LIFE_SV)
        }
        (RadiobiologicalTarget::RadioresistantExtremophile, RadiationExposureRegime::Acute) => {
            RadiationDose::new(D37_ACUTE_EXTREMOPHILE_SV)
        }
        (RadiobiologicalTarget::ComplexEukaryote, RadiationExposureRegime::AnnualChronic) => {
            RadiationDose::new(D37_ANNUAL_CHRONIC_COMPLEX_LIFE_SV)
        }
        (
            RadiobiologicalTarget::RadioresistantExtremophile,
            RadiationExposureRegime::AnnualChronic,
        ) => RadiationDose::new(D37_ANNUAL_CHRONIC_EXTREMOPHILE_SV),
    }
}

pub fn single_hit_survival_fraction(
    dose: RadiationDose,
    d37_reference: RadiationDose,
) -> f64 {
    let d = dose.value();
    let d37 = d37_reference.value();

    if !d.is_finite() || d <= 0.0 {
        return 1.0;
    }

    if !d37.is_finite() || d37 <= 0.0 {
        return 0.0;
    }

    let exponent = -d / d37;
    if exponent < -700.0 {
        0.0
    } else {
        exponent.exp().clamp(0.0, 1.0)
    }
}

pub fn acute_survival_fraction(dose: RadiationDose, target: RadiobiologicalTarget) -> f64 {
    let d37 = d37_reference_dose(target, RadiationExposureRegime::Acute);
    single_hit_survival_fraction(dose, d37)
}

pub fn annual_chronic_survival_fraction(
    annual_dose: RadiationDose,
    target: RadiobiologicalTarget,
) -> f64 {
    let d37 = d37_reference_dose(target, RadiationExposureRegime::AnnualChronic);
    single_hit_survival_fraction(annual_dose, d37)
}

pub fn chronic_to_acute_equivalent_dose(
    annual_dose_rate: RadiationDose,
    exposure_duration: Duration,
) -> RadiationDose {
    let rate = annual_dose_rate.value();
    let duration_seconds = exposure_duration.value();

    if !rate.is_finite() || rate <= 0.0 || !duration_seconds.is_finite() || duration_seconds <= 0.0 {
        return RadiationDose::new(0.0);
    }

    let years = duration_seconds / SECONDS_PER_YEAR;
    RadiationDose::new(rate * years)
}

pub fn evaluate_annual_radiation_tolerance(
    annual_dose: RadiationDose,
) -> RadiationToleranceAssessment {
    let s_complex =
        annual_chronic_survival_fraction(annual_dose, RadiobiologicalTarget::ComplexEukaryote);
    let s_extremophile = annual_chronic_survival_fraction(
        annual_dose,
        RadiobiologicalTarget::RadioresistantExtremophile,
    );

    let threshold = 1.0 / std::f64::consts::E;
    let is_complex_viable = s_complex >= threshold;
    let is_extremophile_viable = s_extremophile >= threshold;

    RadiationToleranceAssessment::new(
        annual_dose,
        s_complex,
        s_extremophile,
        is_complex_viable,
        is_extremophile_viable,
    )
}
