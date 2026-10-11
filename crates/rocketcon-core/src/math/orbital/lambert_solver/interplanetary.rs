use super::solver::solve_lambert;
use super::types::{ PorkchopPoint, TransferDirection };
use crate::error::{ RocketDomainError, RocketDomainResult };
use astronomicon_core::units::{
    Duration,
    GravitationalParameter,
    Length,
    Position,
    Speed,
    VelocityVector,
};

pub fn compute_porkchop_point(
    departure_position: Position,
    departure_body_velocity: VelocityVector,
    arrival_position: Position,
    arrival_body_velocity: VelocityVector,
    time_of_flight: Duration,
    mu_central: GravitationalParameter,
    direction: TransferDirection
) -> RocketDomainResult<PorkchopPoint> {
    let solution = solve_lambert(
        departure_position,
        arrival_position,
        time_of_flight,
        mu_central,
        direction
    )?;

    let v_dep_rel = solution.departure_velocity.raw() - departure_body_velocity.raw();
    let v_arr_rel = solution.arrival_velocity.raw() - arrival_body_velocity.raw();

    let c3 = v_dep_rel.dot(&v_dep_rel);
    let v_inf_dep = c3.sqrt();
    let v_inf_arr = v_arr_rel.magnitude();
    let total_dv = v_inf_dep + v_inf_arr;

    Ok(PorkchopPoint {
        departure_excess_speed: Speed::new(v_inf_dep),
        arrival_excess_speed: Speed::new(v_inf_arr),
        characteristic_energy_c3: c3,
        total_delta_v: Speed::new(total_dv),
        time_of_flight,
        solution,
    })
}

pub fn interplanetary_injection_delta_v(
    departure_body_parking_radius: Length,
    departure_body_mu: GravitationalParameter,
    v_infinity_departure: Speed
) -> Speed {
    let r_park = departure_body_parking_radius.value();
    let mu = departure_body_mu.value();
    let v_inf = v_infinity_departure.value();

    if
        r_park <= 0.0 ||
        mu <= 0.0 ||
        !r_park.is_finite() ||
        !mu.is_finite() ||
        !v_inf.is_finite() ||
        v_inf < 0.0
    {
        return Speed::new(0.0);
    }

    let v_park = (mu / r_park).sqrt();
    let v_inj = (v_inf * v_inf + (2.0 * mu) / r_park).sqrt();
    Speed::new((v_inj - v_park).max(0.0))
}

pub fn interplanetary_capture_delta_v(
    arrival_body_target_periapsis: Length,
    arrival_body_target_apoapsis: Option<Length>,
    arrival_body_mu: GravitationalParameter,
    v_infinity_arrival: Speed
) -> RocketDomainResult<Speed> {
    let r_p = arrival_body_target_periapsis.value();
    let mu = arrival_body_mu.value();
    let v_inf = v_infinity_arrival.value();

    if
        r_p <= 0.0 ||
        mu <= 0.0 ||
        !r_p.is_finite() ||
        !mu.is_finite() ||
        !v_inf.is_finite() ||
        v_inf < 0.0
    {
        return Err(RocketDomainError::InvalidInvariant {
            field: "parameters".to_string(),
            reason: "parameters must be positive and finite".to_string(),
        });
    }

    let r_a = arrival_body_target_apoapsis.map_or(r_p, |a| a.value());
    if r_a < r_p || !r_a.is_finite() {
        return Err(RocketDomainError::InvalidInvariant {
            field: "arrival_body_target_apoapsis".to_string(),
            reason: "apoapsis must be greater than or equal to periapsis".to_string(),
        });
    }

    let v_hyp_p = (v_inf * v_inf + (2.0 * mu) / r_p).sqrt();
    let a_target = 0.5 * (r_p + r_a);
    let v_target_p = (mu * (2.0 / r_p - 1.0 / a_target)).sqrt();
    let delta_v = (v_hyp_p - v_target_p).max(0.0);

    Ok(Speed::new(delta_v))
}
