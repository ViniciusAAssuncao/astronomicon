use crate::error::AppResult;
use crate::hierarchy::load_planet_hierarchy;
use astronomicon_core::math::gravity::{
    calculate_parent_effective_mass, combined_gravitational_parameter,
};
use astronomicon_core::units::Duration;
use chronicon_core::domain::{
    analyze_day_in_month_intercalation, analyze_day_in_year_intercalation,
    analyze_month_in_year_intercalation, analyze_moon_system, analyze_multi_star_system,
    analyze_planetary_day, analyze_planetary_year_with_hierarchy, analyze_seasons,
    DayConvention, IntercalationAnalysis, MoonSystemAnalysis, MultiStarSystemInfo,
    PlanetaryDayInfo, PlanetaryYearInfo, PolysolarDayAnalysis, SeasonalStructure,
    YearConvention,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalendarSkeleton {
    pub planet_id: Uuid,
    pub planet_name: String,
    pub day_info: PlanetaryDayInfo,
    pub year_info: PlanetaryYearInfo,
    pub seasonal_structure: SeasonalStructure,
    pub moon_system: MoonSystemAnalysis,
    pub multi_star: MultiStarSystemInfo,
    pub polysolar_day: PolysolarDayAnalysis,
}

impl CalendarSkeleton {
    pub fn intercalation_day_in_year(
        &self,
        day_convention: DayConvention,
        year_convention: YearConvention,
    ) -> Option<IntercalationAnalysis> {
        let day = self.day_duration(day_convention)?;
        let year = self.year_duration(year_convention)?;
        analyze_day_in_year_intercalation(day, year, None, None)
    }

    pub fn intercalation_month_in_year(
        &self,
        moon_id: &Uuid,
        year_convention: YearConvention,
    ) -> Option<IntercalationAnalysis> {
        let moon = self.moon_system.moons().iter().find(|m| m.moon_id() == *moon_id)?;
        let synodic = moon.synodic_month()?;
        let year = self.year_duration(year_convention)?;
        analyze_month_in_year_intercalation(synodic, year, None, None)
    }

    pub fn intercalation_day_in_month(
        &self,
        day_convention: DayConvention,
        moon_id: &Uuid,
    ) -> Option<IntercalationAnalysis> {
        let day = self.day_duration(day_convention)?;
        let moon = self.moon_system.moons().iter().find(|m| m.moon_id() == *moon_id)?;
        let synodic = moon.synodic_month()?;
        analyze_day_in_month_intercalation(day, synodic, None, None)
    }

    pub fn day_duration(&self, convention: DayConvention) -> Option<Duration> {
        match convention {
            DayConvention::Solar => self.day_info.solar_day(),
            DayConvention::Sidereal => self.day_info.sidereal_day(),
        }
    }

    pub fn year_duration(&self, convention: YearConvention) -> Option<Duration> {
        match convention {
            YearConvention::Sidereal => self.year_info.sidereal_year(),
            YearConvention::Tropical => self.year_info.tropical_year(),
            YearConvention::Anomalistic => self.year_info.anomalistic_year(),
        }
    }

    pub fn polysolar_day(&self) -> &PolysolarDayAnalysis {
        &self.polysolar_day
    }
}

pub async fn resolve_calendar_skeleton(
    pool: &SqlitePool,
    planet_id: &Uuid,
) -> AppResult<CalendarSkeleton> {
    let hierarchy = load_planet_hierarchy(pool, planet_id).await?;

    let stars_map = hierarchy.stars_ref_map();
    let planets_map = hierarchy.planets_ref_map();
    let barycenters_map = hierarchy.barycenters_ref_map();
    let minor_planets_map = hierarchy.minor_planets_ref_map();

    let parent_mass = calculate_parent_effective_mass(
        &hierarchy.target_planet.orbital_parent(),
        &stars_map,
        &planets_map,
        &barycenters_map,
        &minor_planets_map,
    )?;

    let parent_mu = combined_gravitational_parameter(hierarchy.target_planet.mass(), parent_mass);

    let perturbers = hierarchy.satellite_perturbers();

    let day_info = analyze_planetary_day(
        &hierarchy.target_planet,
        parent_mu,
        &hierarchy.target_elements,
    );

    let year_info = analyze_planetary_year_with_hierarchy(
        &hierarchy.target_planet,
        &hierarchy.target_elements,
        &hierarchy.target_planet.orbital_parent(),
        &stars_map,
        &planets_map,
        &barycenters_map,
        &minor_planets_map,
        &perturbers,
    )?;

    let orbital_period = year_info
        .sidereal_year()
        .or(day_info.orbital_period())
        .unwrap_or(Duration::new(0.0));

    let is_tidally_locked = day_info.classification().is_synchronous();

    let seasonal_structure = analyze_seasons(
        &hierarchy.target_planet,
        &hierarchy.target_elements,
        orbital_period,
        is_tidally_locked,
    );

    let satellite_inputs = hierarchy.satellite_inputs();
    let moon_system = analyze_moon_system(
        &hierarchy.target_planet,
        &satellite_inputs,
        year_info.sidereal_year(),
        None,
        None,
    );

    let multi_star = analyze_multi_star_system(
        &hierarchy.target_planet,
        year_info.sidereal_year(),
        &stars_map,
        &planets_map,
        &barycenters_map,
    )?;

    let polysolar_day = hierarchy.analyze_polysolar_day(None)?;

    Ok(CalendarSkeleton {
        planet_id: *planet_id,
        planet_name: hierarchy.target_planet.name().to_string(),
        day_info,
        year_info,
        seasonal_structure,
        moon_system,
        multi_star,
        polysolar_day,
    })
}