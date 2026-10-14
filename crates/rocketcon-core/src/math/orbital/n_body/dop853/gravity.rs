use super::solver::Dop853Error;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GravitySource {
    pub body_id: Uuid,
    pub gravitational_parameter_m3_s2: f64,
    pub position_m: [f64; 3],
}

pub fn inertial_point_mass_derivative<const SOURCES: usize, E>(
    seconds: f64,
    state: &[f64; 6],
    mut ephemeris: E,
) -> Result<[f64; 6], Dop853Error>
where
    E: FnMut(f64) -> Result<[GravitySource; SOURCES], Dop853Error>,
{
    if SOURCES == 0 || !seconds.is_finite() || !state.iter().all(|value| value.is_finite()) {
        return Err(Dop853Error::InvalidGravitySource);
    }
    let sources = ephemeris(seconds)?;
    inertial_point_mass_derivative_from_sources(state, &sources)
}

pub fn inertial_point_mass_derivative_from_sources(
    state: &[f64; 6],
    sources: &[GravitySource],
) -> Result<[f64; 6], Dop853Error> {
    if sources.is_empty() || !state.iter().all(|value| value.is_finite()) {
        return Err(Dop853Error::InvalidGravitySource);
    }
    let mut acceleration = [0.0; 3];
    let mut correction = [0.0; 3];
    for (index, body) in sources.iter().enumerate() {
        if !body.gravitational_parameter_m3_s2.is_finite()
            || body.gravitational_parameter_m3_s2 <= 0.0
            || !body.position_m.iter().all(|value| value.is_finite())
            || sources[..index]
                .iter()
                .any(|other| other.body_id == body.body_id)
        {
            return Err(Dop853Error::InvalidGravitySource);
        }
        let delta = [
            body.position_m[0] - state[0],
            body.position_m[1] - state[1],
            body.position_m[2] - state[2],
        ];
        let squared = delta.iter().map(|x| x * x).sum::<f64>();
        if !squared.is_finite() || squared <= 0.0 {
            return Err(Dop853Error::GravitationalSingularity);
        }
        let factor = body.gravitational_parameter_m3_s2 / (squared * squared.sqrt());
        if !factor.is_finite() {
            return Err(Dop853Error::NumericalBreakdown);
        }
        for axis in 0..3 {
            let term = delta[axis] * factor - correction[axis];
            let combined = acceleration[axis] + term;
            correction[axis] = (combined - acceleration[axis]) - term;
            acceleration[axis] = combined;
        }
    }
    if !acceleration.iter().all(|x| x.is_finite()) {
        return Err(Dop853Error::NumericalBreakdown);
    }
    Ok([
        state[3],
        state[4],
        state[5],
        acceleration[0],
        acceleration[1],
        acceleration[2],
    ])
}
