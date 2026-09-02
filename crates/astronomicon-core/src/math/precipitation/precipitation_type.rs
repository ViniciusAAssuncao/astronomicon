use crate::chemistry::acid_chemistry::{equilibrium_ph_from_dissolved_acids, natural_baseline_ph};
use crate::units::{Pressure, Temperature};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CondensatePrimaryClass {
    AqueousMolecular,
    CryogenicHydrocarbon,
    StrongAcid,
    OtherCovalent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AcidityClassification {
    Neutral,
    NaturalBaseline,
    AcidicRelativeToBaseline,
    StronglyCorrosive,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PrecipitationAcidity {
    primary_class: CondensatePrimaryClass,
    acidity_classification: AcidityClassification,
    ph: Option<f64>,
    natural_baseline_ph: Option<f64>,
}

impl PrecipitationAcidity {
    pub fn new(
        primary_class: CondensatePrimaryClass,
        acidity_classification: AcidityClassification,
        ph: Option<f64>,
        natural_baseline_ph: Option<f64>,
    ) -> Self {
        Self {
            primary_class,
            acidity_classification,
            ph,
            natural_baseline_ph,
        }
    }

    pub fn primary_class(&self) -> CondensatePrimaryClass {
        self.primary_class
    }

    pub fn acidity_classification(&self) -> AcidityClassification {
        self.acidity_classification
    }

    pub fn ph(&self) -> Option<f64> {
        self.ph
    }

    pub fn natural_baseline_ph(&self) -> Option<f64> {
        self.natural_baseline_ph
    }
}

pub fn classify_condensate_formula(formula: &str) -> CondensatePrimaryClass {
    match formula {
        "H2O" => CondensatePrimaryClass::AqueousMolecular,
        "CH4" | "C2H6" | "C3H8" | "C4H10" => CondensatePrimaryClass::CryogenicHydrocarbon,
        "H2SO4" | "HNO3" | "HCl" => CondensatePrimaryClass::StrongAcid,
        _ => CondensatePrimaryClass::OtherCovalent,
    }
}

pub fn dominant_condensate_class(hydrosphere_composition: &[(String, f64)]) -> CondensatePrimaryClass {
    if hydrosphere_composition.is_empty() {
        return CondensatePrimaryClass::OtherCovalent;
    }

    let mut dominant_formula = "";
    let mut max_percentage = -1.0;

    for (formula, percentage) in hydrosphere_composition {
        if *percentage > max_percentage {
            max_percentage = *percentage;
            dominant_formula = formula.as_str();
        }
    }

    classify_condensate_formula(dominant_formula)
}

pub fn evaluate_precipitation_acidity(
    hydrosphere_composition: &[(String, f64)],
    atmospheric_composition: &[(String, f64)],
    surface_pressure: Pressure,
    surface_temperature: Temperature,
) -> PrecipitationAcidity {
    let primary_class = dominant_condensate_class(hydrosphere_composition);

    match primary_class {
        CondensatePrimaryClass::StrongAcid => PrecipitationAcidity::new(
            primary_class,
            AcidityClassification::StronglyCorrosive,
            None,
            None,
        ),
        CondensatePrimaryClass::CryogenicHydrocarbon | CondensatePrimaryClass::OtherCovalent => {
            PrecipitationAcidity::new(primary_class, AcidityClassification::Neutral, None, None)
        }
        CondensatePrimaryClass::AqueousMolecular => {
            let mut acidic_pressures: Vec<(&str, Pressure)> = Vec::new();
            let mut co2_partial_pressure = Pressure::new(0.0);

            for (formula, percentage) in atmospheric_composition {
                let frac = percentage / 100.0;
                if frac <= 0.0 || !frac.is_finite() {
                    continue;
                }

                let p_gas = Pressure::new(surface_pressure.value() * frac);
                match formula.as_str() {
                    "CO2" => {
                        co2_partial_pressure = p_gas;
                        acidic_pressures.push(("CO2", p_gas));
                    }
                    "SO2" => {
                        acidic_pressures.push(("SO2", p_gas));
                    }
                    "NO2" => {
                        acidic_pressures.push(("NO2", p_gas));
                    }
                    "H2S" => {
                        acidic_pressures.push(("H2S", p_gas));
                    }
                    "HCl" => {
                        acidic_pressures.push(("HCl", p_gas));
                    }
                    _ => {}
                }
            }

            let actual_ph =
                equilibrium_ph_from_dissolved_acids(&acidic_pressures, surface_temperature);
            let baseline_ph = natural_baseline_ph(co2_partial_pressure, surface_temperature);

            let acidity_classification = if actual_ph < 3.0 {
                AcidityClassification::StronglyCorrosive
            } else if actual_ph < baseline_ph - 0.3 {
                AcidityClassification::AcidicRelativeToBaseline
            } else if (actual_ph - 7.0).abs() < 0.1 && (baseline_ph - 7.0).abs() < 0.1 {
                AcidityClassification::Neutral
            } else {
                AcidityClassification::NaturalBaseline
            };

            PrecipitationAcidity::new(
                primary_class,
                acidity_classification,
                Some(actual_ph),
                Some(baseline_ph),
            )
        }
    }
}
