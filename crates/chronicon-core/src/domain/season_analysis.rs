use crate::constants::constants::{
    ASEASONAL_ECCENTRICITY_THRESHOLD, ASEASONAL_OBLIQUITY_THRESHOLD_RAD,
    DISTANCE_DRIVEN_ECCENTRICITY_THRESHOLD,
};
use crate::domain::season_classification::SeasonalityClassification;
use crate::math::seasons::{
    cardinal_true_anomalies, climatic_precession_index, mean_anomaly_from_true,
    orbital_duration_between_true_anomalies, time_from_periapsis,
};
use astronomicon_core::domain::{OrbitalElements, Planet};
use astronomicon_core::units::{Angle, Duration};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SeasonCardinalPoints {
    pub north_spring_equinox_true_anomaly: Angle,
    pub north_summer_solstice_true_anomaly: Angle,
    pub north_autumn_equinox_true_anomaly: Angle,
    pub north_winter_solstice_true_anomaly: Angle,
    pub north_spring_equinox_time: Duration,
    pub north_summer_solstice_time: Duration,
    pub north_autumn_equinox_time: Duration,
    pub north_winter_solstice_time: Duration,
}

impl SeasonCardinalPoints {
    pub fn new(
        north_spring_equinox_true_anomaly: Angle,
        north_summer_solstice_true_anomaly: Angle,
        north_autumn_equinox_true_anomaly: Angle,
        north_winter_solstice_true_anomaly: Angle,
        north_spring_equinox_time: Duration,
        north_summer_solstice_time: Duration,
        north_autumn_equinox_time: Duration,
        north_winter_solstice_time: Duration,
    ) -> Self {
        Self {
            north_spring_equinox_true_anomaly,
            north_summer_solstice_true_anomaly,
            north_autumn_equinox_true_anomaly,
            north_winter_solstice_true_anomaly,
            north_spring_equinox_time,
            north_summer_solstice_time,
            north_autumn_equinox_time,
            north_winter_solstice_time,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HemisphericSeasonDurations {
    pub spring: Duration,
    pub summer: Duration,
    pub autumn: Duration,
    pub winter: Duration,
}

impl HemisphericSeasonDurations {
    pub fn new(
        spring: Duration,
        summer: Duration,
        autumn: Duration,
        winter: Duration,
    ) -> Self {
        Self {
            spring,
            summer,
            autumn,
            winter,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SeasonalStructure {
    pub classification: SeasonalityClassification,
    pub cardinal_points: Option<SeasonCardinalPoints>,
    pub northern_hemisphere_durations: Option<HemisphericSeasonDurations>,
    pub southern_hemisphere_durations: Option<HemisphericSeasonDurations>,
    pub climatic_precession_index: f64,
    pub orbital_period: Duration,
}

impl SeasonalStructure {
    pub fn new(
        classification: SeasonalityClassification,
        cardinal_points: Option<SeasonCardinalPoints>,
        northern_hemisphere_durations: Option<HemisphericSeasonDurations>,
        southern_hemisphere_durations: Option<HemisphericSeasonDurations>,
        climatic_precession_index: f64,
        orbital_period: Duration,
    ) -> Self {
        Self {
            classification,
            cardinal_points,
            northern_hemisphere_durations,
            southern_hemisphere_durations,
            climatic_precession_index,
            orbital_period,
        }
    }

    pub fn classification(&self) -> SeasonalityClassification {
        self.classification
    }

    pub fn cardinal_points(&self) -> Option<SeasonCardinalPoints> {
        self.cardinal_points
    }

    pub fn northern_hemisphere_durations(&self) -> Option<HemisphericSeasonDurations> {
        self.northern_hemisphere_durations
    }

    pub fn southern_hemisphere_durations(&self) -> Option<HemisphericSeasonDurations> {
        self.southern_hemisphere_durations
    }

    pub fn climatic_precession_index(&self) -> f64 {
        self.climatic_precession_index
    }

    pub fn orbital_period(&self) -> Duration {
        self.orbital_period
    }
}

pub fn analyze_seasons(
    planet: &Planet,
    orbital_elements: &OrbitalElements,
    orbital_period: Duration,
    is_tidally_locked: bool,
) -> SeasonalStructure {
    let e = orbital_elements.eccentricity();
    let obl = planet.obliquity().map(|a| a.value().abs()).unwrap_or(0.0);
    let nu_sol = planet
        .solstice_true_anomaly()
        .unwrap_or(Angle::new(0.0));

    let varpi = orbital_elements.longitude_of_periapsis();
    let prec_index = climatic_precession_index(e, varpi, nu_sol);

    if is_tidally_locked {
        if e < ASEASONAL_ECCENTRICITY_THRESHOLD {
            return SeasonalStructure::new(
                SeasonalityClassification::TidallyLockedAseasonal,
                None,
                None,
                None,
                prec_index,
                orbital_period,
            );
        } else {
            return SeasonalStructure::new(
                SeasonalityClassification::TidallyLockedDistanceDriven,
                None,
                None,
                None,
                prec_index,
                orbital_period,
            );
        }
    }

    if obl < ASEASONAL_OBLIQUITY_THRESHOLD_RAD && e < ASEASONAL_ECCENTRICITY_THRESHOLD {
        return SeasonalStructure::new(
            SeasonalityClassification::Aseasonal,
            None,
            None,
            None,
            prec_index,
            orbital_period,
        );
    }

    let classification = if obl < ASEASONAL_OBLIQUITY_THRESHOLD_RAD
        && e >= DISTANCE_DRIVEN_ECCENTRICITY_THRESHOLD
    {
        SeasonalityClassification::EccentricityDominated
    } else if obl >= ASEASONAL_OBLIQUITY_THRESHOLD_RAD && e < ASEASONAL_ECCENTRICITY_THRESHOLD {
        SeasonalityClassification::ObliquityDominated
    } else {
        SeasonalityClassification::Combined
    };

    let (nu_n_sp, nu_n_su, nu_n_au, nu_n_wi) = cardinal_true_anomalies(nu_sol);

    let m_n_sp = mean_anomaly_from_true(nu_n_sp, e);
    let m_n_su = mean_anomaly_from_true(nu_n_su, e);
    let m_n_au = mean_anomaly_from_true(nu_n_au, e);
    let m_n_wi = mean_anomaly_from_true(nu_n_wi, e);

    let t_n_sp = time_from_periapsis(m_n_sp, orbital_period);
    let t_n_su = time_from_periapsis(m_n_su, orbital_period);
    let t_n_au = time_from_periapsis(m_n_au, orbital_period);
    let t_n_wi = time_from_periapsis(m_n_wi, orbital_period);

    let cardinal_points = SeasonCardinalPoints::new(
        nu_n_sp, nu_n_su, nu_n_au, nu_n_wi, t_n_sp, t_n_su, t_n_au, t_n_wi,
    );

    let dur_n_sp = orbital_duration_between_true_anomalies(nu_n_sp, nu_n_su, e, orbital_period);
    let dur_n_su = orbital_duration_between_true_anomalies(nu_n_su, nu_n_au, e, orbital_period);
    let dur_n_au = orbital_duration_between_true_anomalies(nu_n_au, nu_n_wi, e, orbital_period);
    let dur_n_wi = orbital_duration_between_true_anomalies(nu_n_wi, nu_n_sp, e, orbital_period);

    let north_durations =
        HemisphericSeasonDurations::new(dur_n_sp, dur_n_su, dur_n_au, dur_n_wi);

    let south_durations =
        HemisphericSeasonDurations::new(dur_n_au, dur_n_wi, dur_n_sp, dur_n_su);

    SeasonalStructure::new(
        classification,
        Some(cardinal_points),
        Some(north_durations),
        Some(south_durations),
        prec_index,
        orbital_period,
    )
}