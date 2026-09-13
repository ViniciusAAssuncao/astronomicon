use crate::constants::constants::{
    DEFAULT_MAX_RESONANCE_DEVIATION, DEFAULT_MAX_RESONANCE_ORDER,
    DEFAULT_MAX_SAROS_SEARCH_MONTHS, DEFAULT_SAROS_ANOMALISTIC_TOLERANCE,
    DEFAULT_SAROS_DRACONIC_TOLERANCE,
};
use crate::math::eclipse_cycle::{find_saros_like_cycle, SarosLikeCycle};
use crate::math::lunar_cycles::{calculate_lunar_months, LunarMonths};
use crate::math::moon_resonance::{
    find_all_pair_resonances, find_laplace_trio_resonances, LaplaceTrioResonance,
    MoonPairResonance,
};
use astronomicon_core::domain::{OrbitalElements, Planet};
use astronomicon_core::math::gravity::combined_gravitational_parameter;
use astronomicon_core::math::kepler::mean_motion;
use astronomicon_core::units::{Duration, Length, Mass};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoonMonthInfo {
    pub moon_id: Uuid,
    pub moon_name: String,
    pub months: LunarMonths,
    pub saros_cycle: Option<SarosLikeCycle>,
}

impl MoonMonthInfo {
    pub fn new(
        moon_id: Uuid,
        moon_name: impl Into<String>,
        months: LunarMonths,
        saros_cycle: Option<SarosLikeCycle>,
    ) -> Self {
        Self {
            moon_id,
            moon_name: moon_name.into(),
            months,
            saros_cycle,
        }
    }

    pub fn moon_id(&self) -> Uuid {
        self.moon_id
    }

    pub fn moon_name(&self) -> &str {
        &self.moon_name
    }

    pub fn months(&self) -> &LunarMonths {
        &self.months
    }

    pub fn sidereal_month(&self) -> Option<Duration> {
        self.months.sidereal
    }

    pub fn synodic_month(&self) -> Option<Duration> {
        self.months.synodic
    }

    pub fn anomalistic_month(&self) -> Option<Duration> {
        self.months.anomalistic
    }

    pub fn draconic_month(&self) -> Option<Duration> {
        self.months.draconic
    }

    pub fn is_retrograde_orbit(&self) -> bool {
        self.months.is_retrograde_orbit
    }

    pub fn saros_cycle(&self) -> Option<SarosLikeCycle> {
        self.saros_cycle
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoonSystemAnalysis {
    pub host_planet_id: Uuid,
    pub host_planet_name: String,
    pub moons: Vec<MoonMonthInfo>,
    pub pair_resonances: Vec<MoonPairResonance>,
    pub laplace_resonances: Vec<LaplaceTrioResonance>,
}

impl MoonSystemAnalysis {
    pub fn new(
        host_planet_id: Uuid,
        host_planet_name: impl Into<String>,
        moons: Vec<MoonMonthInfo>,
        pair_resonances: Vec<MoonPairResonance>,
        laplace_resonances: Vec<LaplaceTrioResonance>,
    ) -> Self {
        Self {
            host_planet_id,
            host_planet_name: host_planet_name.into(),
            moons,
            pair_resonances,
            laplace_resonances,
        }
    }

    pub fn host_planet_id(&self) -> Uuid {
        self.host_planet_id
    }

    pub fn host_planet_name(&self) -> &str {
        &self.host_planet_name
    }

    pub fn moons(&self) -> &[MoonMonthInfo] {
        &self.moons
    }

    pub fn pair_resonances(&self) -> &[MoonPairResonance] {
        &self.pair_resonances
    }

    pub fn laplace_resonances(&self) -> &[LaplaceTrioResonance] {
        &self.laplace_resonances
    }
}

#[derive(Debug, Clone)]
pub struct SatelliteInput<'a> {
    pub id: Uuid,
    pub name: &'a str,
    pub mass: Mass,
    pub orbital_elements: &'a OrbitalElements,
}

impl<'a> SatelliteInput<'a> {
    pub fn new(
        id: Uuid,
        name: &'a str,
        mass: Mass,
        orbital_elements: &'a OrbitalElements,
    ) -> Self {
        Self {
            id,
            name,
            mass,
            orbital_elements,
        }
    }
}

pub fn analyze_single_moon(
    moon_id: Uuid,
    moon_name: &str,
    moon_mass: Mass,
    moon_orbital_elements: &OrbitalElements,
    host_planet_mass: Mass,
    host_planet_j2: Option<f64>,
    host_planet_equatorial_radius: Option<Length>,
    planet_orbital_period: Option<Duration>,
    max_saros_search: Option<u32>,
    saros_draconic_tolerance: Option<f64>,
    saros_anomalistic_tolerance: Option<f64>,
) -> MoonMonthInfo {
    let months = calculate_lunar_months(
        moon_orbital_elements.semi_major_axis(),
        moon_orbital_elements.eccentricity(),
        moon_orbital_elements.inclination(),
        moon_mass,
        host_planet_mass,
        host_planet_j2,
        host_planet_equatorial_radius,
        planet_orbital_period,
    );

    let saros_cycle = match (months.synodic, months.draconic, months.anomalistic) {
        (Some(syn), Some(drac), Some(anom)) => find_saros_like_cycle(
            syn,
            drac,
            anom,
            max_saros_search.or(Some(DEFAULT_MAX_SAROS_SEARCH_MONTHS)),
            saros_draconic_tolerance.or(Some(DEFAULT_SAROS_DRACONIC_TOLERANCE)),
            saros_anomalistic_tolerance.or(Some(DEFAULT_SAROS_ANOMALISTIC_TOLERANCE)),
        ),
        _ => None,
    };

    MoonMonthInfo::new(moon_id, moon_name, months, saros_cycle)
}

pub fn analyze_moon_from_planet(
    moon_planet: &Planet,
    moon_orbital_elements: &OrbitalElements,
    host_planet: &Planet,
    planet_orbital_period: Option<Duration>,
) -> MoonMonthInfo {
    analyze_single_moon(
        moon_planet.id(),
        moon_planet.name(),
        moon_planet.mass(),
        moon_orbital_elements,
        host_planet.mass(),
        host_planet.oblateness_j2(),
        host_planet.equatorial_radius(),
        planet_orbital_period,
        None,
        None,
        None,
    )
}

pub fn analyze_moon_system(
    host_planet: &Planet,
    satellites: &[SatelliteInput],
    planet_orbital_period: Option<Duration>,
    max_resonance_order: Option<u32>,
    max_resonance_deviation: Option<f64>,
) -> MoonSystemAnalysis {
    let mut moon_infos = Vec::with_capacity(satellites.len());
    let mut moon_mean_motions = Vec::with_capacity(satellites.len());

    let host_mass = host_planet.mass();
    let host_j2 = host_planet.oblateness_j2();
    let host_r_eq = host_planet.equatorial_radius();

    for sat in satellites {
        let info = analyze_single_moon(
            sat.id,
            sat.name,
            sat.mass,
            sat.orbital_elements,
            host_mass,
            host_j2,
            host_r_eq,
            planet_orbital_period,
            None,
            None,
            None,
        );
        let mu = combined_gravitational_parameter(host_mass, sat.mass);
        let n = mean_motion(sat.orbital_elements.semi_major_axis(), mu);

        moon_mean_motions.push((sat.id, n));
        moon_infos.push(info);
    }

    let order = max_resonance_order.or(Some(DEFAULT_MAX_RESONANCE_ORDER));
    let dev = max_resonance_deviation.or(Some(DEFAULT_MAX_RESONANCE_DEVIATION));

    let pair_resonances = find_all_pair_resonances(&moon_mean_motions, order, dev);
    let laplace_resonances = find_laplace_trio_resonances(&pair_resonances, &moon_mean_motions);

    MoonSystemAnalysis::new(
        host_planet.id(),
        host_planet.name(),
        moon_infos,
        pair_resonances,
        laplace_resonances,
    )
}
