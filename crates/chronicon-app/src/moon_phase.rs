use astronomicon_core::units::{Angle, Duration};
use chronicon_core::domain::MoonMonthInfo;
use chronicon_core::math::SarosLikeCycle;
use serde::{Deserialize, Serialize};
use std::f64::consts::{FRAC_PI_4, FRAC_PI_8, PI, TAU};
use uuid::Uuid;

const DEFAULT_ECLIPSE_SEASON_NODE_LIMIT_RAD: f64 = 0.3228859;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LunarPhaseName {
    NewMoon,
    WaxingCrescent,
    FirstQuarter,
    WaxingGibbous,
    FullMoon,
    WaningGibbous,
    LastQuarter,
    WaningCrescent,
}

impl LunarPhaseName {
    pub fn is_waxing(&self) -> bool {
        matches!(
            self,
            Self::WaxingCrescent | Self::FirstQuarter | Self::WaxingGibbous
        )
    }

    pub fn is_waning(&self) -> bool {
        matches!(
            self,
            Self::WaningGibbous | Self::LastQuarter | Self::WaningCrescent
        )
    }

    pub fn is_syzygy(&self) -> bool {
        matches!(self, Self::NewMoon | Self::FullMoon)
    }

    pub fn is_quadrature(&self) -> bool {
        matches!(self, Self::FirstQuarter | Self::LastQuarter)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoonPhaseInfo {
    pub moon_id: Uuid,
    pub moon_name: String,
    pub phase_angle: Angle,
    pub illumination_fraction: f64,
    pub phase_name: LunarPhaseName,
    pub synodic_month: Option<Duration>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EclipseProximityInfo {
    pub moon_id: Uuid,
    pub moon_name: String,
    pub saros_cycle: Option<SarosLikeCycle>,
    pub is_in_eclipse_season: bool,
    pub node_distance_angle: Option<Angle>,
    pub saros_cycle_progress: Option<f64>,
}

pub fn compute_moon_phase(moon: &MoonMonthInfo, elapsed_since_epoch: Duration) -> MoonPhaseInfo {
    let t_syn = moon.synodic_month().map(|d| d.value()).unwrap_or(0.0);

    if t_syn <= 0.0 || !t_syn.is_finite() {
        return MoonPhaseInfo {
            moon_id: moon.moon_id(),
            moon_name: moon.moon_name().to_string(),
            phase_angle: Angle::new(0.0),
            illumination_fraction: 0.0,
            phase_name: LunarPhaseName::NewMoon,
            synodic_month: moon.synodic_month(),
        };
    }

    let phase_rad = (TAU * (elapsed_since_epoch.value() / t_syn)).rem_euclid(TAU);
    let illumination = ((1.0 - phase_rad.cos()) / 2.0).clamp(0.0, 1.0);

    let offset_phase = (phase_rad + FRAC_PI_8).rem_euclid(TAU);
    let octant = ((offset_phase / FRAC_PI_4).floor() as u32) % 8;

    let phase_name = match octant {
        0 => LunarPhaseName::NewMoon,
        1 => LunarPhaseName::WaxingCrescent,
        2 => LunarPhaseName::FirstQuarter,
        3 => LunarPhaseName::WaxingGibbous,
        4 => LunarPhaseName::FullMoon,
        5 => LunarPhaseName::WaningGibbous,
        6 => LunarPhaseName::LastQuarter,
        _ => LunarPhaseName::WaningCrescent,
    };

    MoonPhaseInfo {
        moon_id: moon.moon_id(),
        moon_name: moon.moon_name().to_string(),
        phase_angle: Angle::new(phase_rad),
        illumination_fraction: illumination,
        phase_name,
        synodic_month: moon.synodic_month(),
    }
}

pub fn compute_eclipse_proximity(
    moon: &MoonMonthInfo,
    elapsed_since_epoch: Duration,
) -> EclipseProximityInfo {
    let t_drac = moon.draconic_month().map(|d| d.value()).unwrap_or(0.0);

    let (node_distance_angle, is_in_eclipse_season) = if t_drac > 0.0 && t_drac.is_finite() {
        let theta_drac = (TAU * (elapsed_since_epoch.value() / t_drac)).rem_euclid(TAU);
        let dist = (theta_drac.rem_euclid(PI)).min(PI - (theta_drac.rem_euclid(PI)));
        (
            Some(Angle::new(dist)),
            dist <= DEFAULT_ECLIPSE_SEASON_NODE_LIMIT_RAD,
        )
    } else {
        (None, false)
    };

    let saros_cycle = moon.saros_cycle();
    let saros_cycle_progress = saros_cycle.and_then(|saros| {
        let t_saros = saros.duration.value();
        if t_saros > 0.0 && t_saros.is_finite() {
            Some((elapsed_since_epoch.value().rem_euclid(t_saros)) / t_saros)
        } else {
            None
        }
    });

    EclipseProximityInfo {
        moon_id: moon.moon_id(),
        moon_name: moon.moon_name().to_string(),
        saros_cycle,
        is_in_eclipse_season,
        node_distance_angle,
        saros_cycle_progress,
    }
}
