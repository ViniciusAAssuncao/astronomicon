use crate::math::axial_precession::{
    axial_precession_period, axial_precession_rate, dynamical_ellipticity, maccullagh_difference,
    AxialPerturber,
};
use crate::math::moment_of_inertia::{
    core_radius_from_mantle_density, two_layer_polar_moment_of_inertia,
};
use crate::math::orbital_year::{
    anomalistic_year_length, sidereal_year_length, tropical_year_length,
};
use astronomicon_core::domain::{
    Barycenter, MinorPlanet, OrbitalElements, OrbitalParent, Planet, PlanetRheology, Star,
};
use astronomicon_core::error::DomainResult;
use astronomicon_core::math::gravity::calculate_parent_effective_mass;
use astronomicon_core::math::gravity::combined_gravitational_parameter;
use astronomicon_core::math::perturbation::resolve_secular_precession;
use astronomicon_core::math::rotation::angular_velocity_from_rotation_period;
use astronomicon_core::units::{
    Angle, AngularVelocity, Duration, GravitationalParameter, Length, Mass,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlanetaryYearInfo {
    pub sidereal_year: Option<Duration>,
    pub anomalistic_year: Option<Duration>,
    pub tropical_year: Option<Duration>,
    pub axial_precession_rate: AngularVelocity,
    pub axial_precession_period: Option<Duration>,
    pub has_precession_data: bool,
}

impl PlanetaryYearInfo {
    pub fn new(
        sidereal_year: Option<Duration>,
        anomalistic_year: Option<Duration>,
        tropical_year: Option<Duration>,
        axial_precession_rate: AngularVelocity,
        axial_precession_period: Option<Duration>,
        has_precession_data: bool,
    ) -> Self {
        Self {
            sidereal_year,
            anomalistic_year,
            tropical_year,
            axial_precession_rate,
            axial_precession_period,
            has_precession_data,
        }
    }

    pub fn sidereal_year(&self) -> Option<Duration> {
        self.sidereal_year
    }

    pub fn anomalistic_year(&self) -> Option<Duration> {
        self.anomalistic_year
    }

    pub fn tropical_year(&self) -> Option<Duration> {
        self.tropical_year
    }

    pub fn axial_precession_rate(&self) -> AngularVelocity {
        self.axial_precession_rate
    }

    pub fn axial_precession_period(&self) -> Option<Duration> {
        self.axial_precession_period
    }

    pub fn has_precession_data(&self) -> bool {
        self.has_precession_data
    }
}

pub fn analyze_planetary_year(
    planet: &Planet,
    orbital_elements: &OrbitalElements,
    parent_mass: Mass,
    parent_mu: GravitationalParameter,
    parent_j2: Option<f64>,
    parent_equatorial_radius: Option<Length>,
    additional_perturbers: &[AxialPerturber],
) -> PlanetaryYearInfo {
    let t_sid = sidereal_year_length(orbital_elements.semi_major_axis(), parent_mu);

    let sidereal = match t_sid {
        Some(t) if t.value() > 0.0 && t.value().is_finite() => t,
        _ => {
            return PlanetaryYearInfo::new(
                None,
                None,
                None,
                AngularVelocity::new(0.0),
                None,
                false,
            );
        }
    };

    let secular_rates = resolve_secular_precession(
        orbital_elements,
        parent_mu,
        parent_j2,
        parent_equatorial_radius,
    );

    let t_anom = anomalistic_year_length(
        orbital_elements.semi_major_axis(),
        parent_mu,
        secular_rates.apsidal,
    );

    let j2_opt = planet.oblateness_j2();
    let r_eq_opt = planet.equatorial_radius();
    let rot_opt = planet.rotation_period();

    let (rate_prec, period_prec, t_trop, has_data) = match (j2_opt, r_eq_opt, rot_opt) {
        (Some(j2), Some(r_eq), Some(rot)) if j2 > 0.0 && r_eq.value() > 0.0 && rot.value() > 0.0 => {
            let cmf = planet.core_mass_fraction().unwrap_or(0.0);
            let mantle_density = planet
                .rheology()
                .map(|r| r.mean_density())
                .unwrap_or_else(|| PlanetRheology::fallback_for_kind(planet.kind()).mean_density());

            let r_core = core_radius_from_mantle_density(planet.mass(), r_eq, cmf, mantle_density);
            let c_polar = two_layer_polar_moment_of_inertia(planet.mass(), r_eq, cmf, r_core);
            let c_minus_a = maccullagh_difference(j2, planet.mass(), r_eq);
            let h_d = dynamical_ellipticity(c_minus_a, c_polar);

            let omega_spin = angular_velocity_from_rotation_period(rot);
            let obliquity = planet.obliquity().unwrap_or(Angle::new(0.0));

            let mut all_perturbers = Vec::with_capacity(1 + additional_perturbers.len());
            all_perturbers.push(AxialPerturber::new(
                parent_mass,
                orbital_elements.semi_major_axis(),
                orbital_elements.eccentricity(),
            ));
            all_perturbers.extend_from_slice(additional_perturbers);

            let rate = axial_precession_rate(h_d, omega_spin, obliquity, &all_perturbers);
            let period = axial_precession_period(rate);
            let trop = tropical_year_length(sidereal, rate).or(Some(sidereal));

            (rate, period, trop, true)
        }
        _ => (
            AngularVelocity::new(0.0),
            None,
            Some(sidereal),
            false,
        ),
    };

    PlanetaryYearInfo::new(
        Some(sidereal),
        t_anom,
        t_trop,
        rate_prec,
        period_prec,
        has_data,
    )
}

pub fn analyze_planetary_year_with_hierarchy(
    planet: &Planet,
    orbital_elements: &OrbitalElements,
    parent: &OrbitalParent,
    stars: &HashMap<Uuid, &Star>,
    planets: &HashMap<Uuid, &Planet>,
    barycenters: &HashMap<Uuid, &Barycenter>,
    minor_planets: &HashMap<Uuid, &MinorPlanet>,
    additional_perturbers: &[AxialPerturber],
) -> DomainResult<PlanetaryYearInfo> {
    let parent_mass =
        calculate_parent_effective_mass(parent, stars, planets, barycenters, minor_planets)?;

    let parent_mu = combined_gravitational_parameter(planet.mass(), parent_mass);

    let (parent_j2, parent_r_eq) = match parent {
        OrbitalParent::Star(id) => {
            if let Some(star) = stars.get(id) {
                (star.oblateness_j2(), star.radius())
            } else {
                (None, None)
            }
        }
        OrbitalParent::Planet(id) => {
            if let Some(parent_planet) = planets.get(id) {
                (parent_planet.oblateness_j2(), parent_planet.equatorial_radius())
            } else {
                (None, None)
            }
        }
        _ => (None, None),
    };

    Ok(analyze_planetary_year(
        planet,
        orbital_elements,
        parent_mass,
        parent_mu,
        parent_j2,
        parent_r_eq,
        additional_perturbers,
    ))
}