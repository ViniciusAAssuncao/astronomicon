use crate::units::Luminosity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SurfaceHabitabilityTier {
    HyperHabitable,
    HabitableMesophilic,
    MarginallyHabitableExtremophilic,
    TransientOrPrebiotic,
    InhabitableSurface,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubsurfaceHabitabilityTier {
    ActiveOceanWorld,
    DormantOceanWorld,
    NoSubsurfaceHabitat,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanetaryHabitabilityClassification {
    pub surface_viability_score: f64,
    pub subsurface_viability_score: f64,
    pub surface_tier: SurfaceHabitabilityTier,
    pub subsurface_tier: SubsurfaceHabitabilityTier,
    pub photosynthetic_productivity_index: f64,
    pub chemosynthetic_power: Luminosity,
    pub nutrient_availability_factor: f64,
}

impl PlanetaryHabitabilityClassification {
    pub fn new(
        surface_viability_score: f64,
        subsurface_viability_score: f64,
        surface_tier: SurfaceHabitabilityTier,
        subsurface_tier: SubsurfaceHabitabilityTier,
        photosynthetic_productivity_index: f64,
        chemosynthetic_power: Luminosity,
        nutrient_availability_factor: f64,
    ) -> Self {
        Self {
            surface_viability_score,
            subsurface_viability_score,
            surface_tier,
            subsurface_tier,
            photosynthetic_productivity_index,
            chemosynthetic_power,
            nutrient_availability_factor,
        }
    }

    pub fn surface_viability_score(&self) -> f64 {
        self.surface_viability_score
    }

    pub fn subsurface_viability_score(&self) -> f64 {
        self.subsurface_viability_score
    }

    pub fn surface_tier(&self) -> SurfaceHabitabilityTier {
        self.surface_tier
    }

    pub fn subsurface_tier(&self) -> SubsurfaceHabitabilityTier {
        self.subsurface_tier
    }

    pub fn photosynthetic_productivity_index(&self) -> f64 {
        self.photosynthetic_productivity_index
    }

    pub fn chemosynthetic_power(&self) -> Luminosity {
        self.chemosynthetic_power
    }

    pub fn nutrient_availability_factor(&self) -> f64 {
        self.nutrient_availability_factor
    }

    pub fn is_surface_habitable(&self) -> bool {
        !matches!(self.surface_tier, SurfaceHabitabilityTier::InhabitableSurface)
    }

    pub fn is_subsurface_habitable(&self) -> bool {
        !matches!(
            self.subsurface_tier,
            SubsurfaceHabitabilityTier::NoSubsurfaceHabitat
        )
    }

    pub fn is_any_regime_habitable(&self) -> bool {
        self.is_surface_habitable() || self.is_subsurface_habitable()
    }
}

pub fn classify_surface_tier(
    surface_viability_score: f64,
    sph_index: f64,
) -> SurfaceHabitabilityTier {
    let score = surface_viability_score.clamp(0.0, 1.0);
    let sph = sph_index.clamp(0.0, 1.0);

    if score >= 0.70 && sph >= 0.50 {
        SurfaceHabitabilityTier::HyperHabitable
    } else if score >= 0.40 && sph >= 0.15 {
        SurfaceHabitabilityTier::HabitableMesophilic
    } else if score >= 0.05 {
        SurfaceHabitabilityTier::MarginallyHabitableExtremophilic
    } else if score > 0.001 {
        SurfaceHabitabilityTier::TransientOrPrebiotic
    } else {
        SurfaceHabitabilityTier::InhabitableSurface
    }
}

pub fn classify_subsurface_tier(
    is_subsurface_ocean: bool,
    chemosynthetic_power: Luminosity,
) -> SubsurfaceHabitabilityTier {
    if !is_subsurface_ocean {
        return SubsurfaceHabitabilityTier::NoSubsurfaceHabitat;
    }

    let p = chemosynthetic_power.value();
    if p >= 1.0e6 {
        SubsurfaceHabitabilityTier::ActiveOceanWorld
    } else {
        SubsurfaceHabitabilityTier::DormantOceanWorld
    }
}

pub fn evaluate_planetary_habitability(
    radiation_survival_fraction: f64,
    chemical_viability_score: f64,
    surface_liquid_solvent_coverage: f64,
    nutrient_availability_factor: f64,
    sph_index: f64,
    is_subsurface_ocean: bool,
    subsurface_chemical_viability_score: f64,
    subsurface_chemosynthetic_power: Luminosity,
) -> PlanetaryHabitabilityClassification {
    let rad = radiation_survival_fraction.clamp(0.0, 1.0);
    let chem_surf = chemical_viability_score.clamp(0.0, 1.0);
    let solv_surf = surface_liquid_solvent_coverage.clamp(0.0, 1.0);
    let nutr = nutrient_availability_factor.clamp(0.0, 1.0);

    let surface_score = (rad * chem_surf * solv_surf * (0.2 + 0.8 * nutr)).clamp(0.0, 1.0);
    let surface_tier = classify_surface_tier(surface_score, sph_index);

    let subsurface_score = if is_subsurface_ocean {
        let chem_sub = subsurface_chemical_viability_score.clamp(0.0, 1.0);
        let p_chem = subsurface_chemosynthetic_power.value().max(0.0);
        let energy_factor = (p_chem / (p_chem + 1.0e8)).sqrt().clamp(0.0, 1.0);
        (chem_sub * energy_factor * (0.2 + 0.8 * nutr)).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let subsurface_tier =
        classify_subsurface_tier(is_subsurface_ocean, subsurface_chemosynthetic_power);

    PlanetaryHabitabilityClassification::new(
        surface_score,
        subsurface_score,
        surface_tier,
        subsurface_tier,
        sph_index.clamp(0.0, 1.0),
        subsurface_chemosynthetic_power,
        nutr,
    )
}
