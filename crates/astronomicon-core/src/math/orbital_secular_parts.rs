use crate::domain::OrbitalElements;
use crate::error::DomainResult;
use crate::math::kepler::{
    mean_motion, perifocal_state_vectors, rotate_perifocal_to_system, solve_kepler,
    true_anomaly_from_eccentric,
};
use crate::math::orbital_phase::compensated_orbital_phase;
use crate::math::perturbation::SecularPrecessionRates;
use crate::units::{Duration, GravitationalParameter, Position};

pub fn orbital_position_secular_at_epoch_parts(
    elements: &OrbitalElements,
    mu: GravitationalParameter,
    secular_rates: &SecularPrecessionRates,
    universe_epoch: Duration,
    local_epoch: Duration,
) -> DomainResult<Position> {
    let mean_rate =
        mean_motion(elements.semi_major_axis(), mu) + secular_rates.mean_anomaly_correction;
    let mean_anomaly = compensated_orbital_phase(
        elements.mean_anomaly_at_epoch(),
        mean_rate,
        universe_epoch,
        local_epoch,
    )?;
    let eccentric_anomaly = solve_kepler(mean_anomaly, elements.eccentricity())?;
    let true_anomaly = true_anomaly_from_eccentric(eccentric_anomaly, elements.eccentricity());
    let (perifocal_position, perifocal_velocity) = perifocal_state_vectors(
        elements.semi_major_axis(),
        elements.eccentricity(),
        true_anomaly,
        mu,
    );
    let argument = compensated_orbital_phase(
        elements.argument_of_periapsis(),
        secular_rates.apsidal,
        universe_epoch,
        local_epoch,
    )?;
    let ascending_node = compensated_orbital_phase(
        elements.longitude_of_ascending_node(),
        secular_rates.nodal,
        universe_epoch,
        local_epoch,
    )?;
    let (position, _) = rotate_perifocal_to_system(
        perifocal_position,
        perifocal_velocity,
        argument,
        elements.inclination(),
        ascending_node,
    );
    Ok(position)
}
