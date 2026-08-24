use crate::chemistry::abundance::ElementalAbundance;
use crate::chemistry::condensation::{condensation_fraction, thermal_condensation_efficiency};
use crate::domain::PlanetKind;
use crate::units::constants::STEFAN_BOLTZMANN_CONSTANT;
use crate::units::{Length, Luminosity, Temperature};
use std::f64::consts::PI;

pub fn protoplanetary_disk_temperature(
    star_luminosity: Luminosity,
    orbital_distance: Length,
    disk_albedo: f64,
) -> Temperature {
    let l = star_luminosity.value();
    let r = orbital_distance.value();
    let a = disk_albedo.clamp(0.0, 1.0);

    if l <= 0.0 || r <= 0.0 || !l.is_finite() || !r.is_finite() {
        return Temperature::new(0.0);
    }

    let num = l * (1.0 - a);
    let den = 16.0 * PI * STEFAN_BOLTZMANN_CONSTANT * r * r;

    if den <= 0.0 || !den.is_finite() {
        return Temperature::new(0.0);
    }

    let t4 = num / den;
    if !t4.is_finite() || t4 <= 0.0 {
        Temperature::new(0.0)
    } else {
        Temperature::new(t4.powf(0.25))
    }
}

pub fn disk_temperature_at_orbit(
    star_luminosity: Luminosity,
    orbital_distance: Length,
) -> Temperature {
    protoplanetary_disk_temperature(star_luminosity, orbital_distance, 0.5)
}

pub fn condensation_fraction_with_tc(
    condensation_temp: Temperature,
    disk_temperature: Temperature,
    transition_width: f64,
) -> f64 {
    thermal_condensation_efficiency(condensation_temp, disk_temperature, transition_width)
}

pub fn planetary_bulk_composition_from_disk_temp(
    star_abundances: &[ElementalAbundance],
    disk_temperature: Temperature,
    planet_kind: PlanetKind,
) -> Vec<ElementalAbundance> {
    let transition_width = 50.0;
    let si_fraction = condensation_fraction("Si", disk_temperature, transition_width);

    let mut raw_fractions = Vec::with_capacity(star_abundances.len());
    let mut total_raw = 0.0;

    for item in star_abundances {
        let sym = item.symbol();
        let w_star = item.mass_fraction();

        if w_star <= 0.0 {
            raw_fractions.push((sym.to_string(), 0.0));
            continue;
        }

        let mut eta = condensation_fraction(sym, disk_temperature, transition_width);

        if sym == "O" {
            eta = eta.max(0.6 * si_fraction);
        }

        let raw_m = match planet_kind {
            PlanetKind::Telluric | PlanetKind::Chthonian => w_star * eta,
            PlanetKind::CarbonPlanet => {
                if sym == "C" {
                    eta = eta.max(si_fraction);
                }
                w_star * eta
            }
            PlanetKind::GasGiant => {
                if sym == "H" || sym == "He" {
                    w_star
                } else {
                    w_star * (0.05 + 0.95 * eta)
                }
            }
            PlanetKind::IceGiant => {
                if sym == "H" || sym == "He" {
                    w_star * 0.08
                } else {
                    w_star * eta
                }
            }
            PlanetKind::IcyBody | PlanetKind::DwarfPlanet => {
                if sym == "H" || sym == "He" {
                    0.0
                } else {
                    w_star * eta
                }
            }
            PlanetKind::Exotic => w_star * eta,
        };

        raw_fractions.push((sym.to_string(), raw_m));
        total_raw += raw_m;
    }

    if total_raw <= 0.0 {
        return star_abundances.to_vec();
    }

    raw_fractions
        .into_iter()
        .map(|(sym, m)| ElementalAbundance::new(sym, m / total_raw))
        .collect()
}

pub fn planetary_bulk_composition(
    star_abundances: &[ElementalAbundance],
    orbital_distance: Length,
    star_luminosity: Luminosity,
    planet_kind: PlanetKind,
) -> Vec<ElementalAbundance> {
    let disk_temp = disk_temperature_at_orbit(star_luminosity, orbital_distance);
    planetary_bulk_composition_from_disk_temp(star_abundances, disk_temp, planet_kind)
}
