use astronomicon_core::units::Duration;
use chronicon_core::domain::{SeasonalStructure, SeasonalityClassification};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SeasonName {
    Spring,
    Summer,
    Autumn,
    Winter,
}

impl SeasonName {
    pub fn opposite(&self) -> Self {
        match self {
            Self::Spring => Self::Autumn,
            Self::Summer => Self::Winter,
            Self::Autumn => Self::Spring,
            Self::Winter => Self::Summer,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeasonState {
    pub classification: SeasonalityClassification,
    pub northern_season: Option<SeasonName>,
    pub southern_season: Option<SeasonName>,
    pub season_progress: f64,
    pub orbital_progress: f64,
}

impl SeasonState {
    pub fn new(
        classification: SeasonalityClassification,
        northern_season: Option<SeasonName>,
        southern_season: Option<SeasonName>,
        season_progress: f64,
        orbital_progress: f64,
    ) -> Self {
        Self {
            classification,
            northern_season,
            southern_season,
            season_progress,
            orbital_progress,
        }
    }

    pub fn aseasonal(classification: SeasonalityClassification, orbital_progress: f64) -> Self {
        Self::new(classification, None, None, 0.0, orbital_progress)
    }
}

pub fn resolve_season_state(
    seasonal_structure: &SeasonalStructure,
    elapsed_since_epoch: Duration,
) -> SeasonState {
    let t_orb = seasonal_structure.orbital_period().value();
    if t_orb <= 0.0 || !t_orb.is_finite() {
        return SeasonState::aseasonal(seasonal_structure.classification(), 0.0);
    }

    let t_norm = elapsed_since_epoch.value().rem_euclid(t_orb);
    let orbital_progress = t_norm / t_orb;

    if seasonal_structure.classification().is_aseasonal() {
        return SeasonState::aseasonal(seasonal_structure.classification(), orbital_progress);
    }

    let (cardinal, north_durations) = match (
        seasonal_structure.cardinal_points(),
        seasonal_structure.northern_hemisphere_durations(),
    ) {
        (Some(cp), Some(nd)) => (cp, nd),
        _ => return SeasonState::aseasonal(seasonal_structure.classification(), orbital_progress),
    };

    let t_sp = cardinal.north_spring_equinox_time.value();
    let d_sp = north_durations.spring.value();
    let d_su = north_durations.summer.value();
    let d_au = north_durations.autumn.value();
    let d_wi = north_durations.winter.value();

    let delta_sp = (t_norm - t_sp).rem_euclid(t_orb);

    if delta_sp < d_sp {
        let prog = if d_sp > 0.0 { delta_sp / d_sp } else { 0.0 };
        SeasonState::new(
            seasonal_structure.classification(),
            Some(SeasonName::Spring),
            Some(SeasonName::Autumn),
            prog.clamp(0.0, 1.0),
            orbital_progress,
        )
    } else if delta_sp < d_sp + d_su {
        let prog = if d_su > 0.0 {
            (delta_sp - d_sp) / d_su
        } else {
            0.0
        };
        SeasonState::new(
            seasonal_structure.classification(),
            Some(SeasonName::Summer),
            Some(SeasonName::Winter),
            prog.clamp(0.0, 1.0),
            orbital_progress,
        )
    } else if delta_sp < d_sp + d_su + d_au {
        let prog = if d_au > 0.0 {
            (delta_sp - d_sp - d_su) / d_au
        } else {
            0.0
        };
        SeasonState::new(
            seasonal_structure.classification(),
            Some(SeasonName::Autumn),
            Some(SeasonName::Spring),
            prog.clamp(0.0, 1.0),
            orbital_progress,
        )
    } else {
        let prog = if d_wi > 0.0 {
            (delta_sp - d_sp - d_su - d_au) / d_wi
        } else {
            0.0
        };
        SeasonState::new(
            seasonal_structure.classification(),
            Some(SeasonName::Winter),
            Some(SeasonName::Summer),
            prog.clamp(0.0, 1.0),
            orbital_progress,
        )
    }
}
