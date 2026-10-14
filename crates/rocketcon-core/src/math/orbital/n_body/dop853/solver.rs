use super::coefficients::{A, B, C, D, E3, E5};
use super::dense::Dop853DenseSegment;

#[derive(Debug, Clone, Copy)]
pub struct Dop853Config<const N: usize> {
    pub absolute_tolerances: [f64; N],
    pub relative_tolerance: f64,
    pub initial_step_s: f64,
    pub minimum_step_s: f64,
    pub maximum_step_s: f64,
    pub maximum_attempts: usize,
    pub maximum_evaluations: usize,
    pub maximum_segments: usize,
}

impl<const N: usize> Dop853Config<N> {
    fn validate(&self) -> Result<(), Dop853Error> {
        if N == 0
            || !self
                .absolute_tolerances
                .iter()
                .all(|x| x.is_finite() && *x > 0.0)
            || !self.relative_tolerance.is_finite()
            || self.relative_tolerance < 0.0
            || ![
                self.initial_step_s,
                self.minimum_step_s,
                self.maximum_step_s,
            ]
            .iter()
            .all(|x| x.is_finite() && *x > 0.0)
            || self.minimum_step_s > self.maximum_step_s
            || self.initial_step_s < self.minimum_step_s
            || self.initial_step_s > self.maximum_step_s
            || self.maximum_attempts == 0
            || self.maximum_evaluations < 16
            || self.maximum_segments == 0
        {
            return Err(Dop853Error::InvalidConfiguration);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Dop853Error {
    InvalidConfiguration,
    InvalidInitialState,
    Derivative(String),
    NonFiniteDerivative,
    NumericalBreakdown,
    InvalidGravitySource,
    GravitationalSingularity,
    StepUnderflow,
    BudgetExceeded,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Dop853Result<const N: usize> {
    pub final_state: [f64; N],
    pub reached_seconds: f64,
    pub segments: Vec<Dop853DenseSegment<N>>,
    pub accepted_steps: usize,
    pub rejected_steps: usize,
    pub evaluations: usize,
    pub event_seconds: Option<f64>,
}

fn evaluate<const N: usize, F, C>(
    f: &mut F,
    cancelled: &mut C,
    seconds: f64,
    state: &[f64; N],
    evaluations: &mut usize,
    budget: usize,
) -> Result<[f64; N], Dop853Error>
where
    F: FnMut(f64, &[f64; N]) -> Result<[f64; N], Dop853Error>,
    C: FnMut() -> bool,
{
    if cancelled() {
        return Err(Dop853Error::Cancelled);
    }
    if *evaluations >= budget {
        return Err(Dop853Error::BudgetExceeded);
    }
    if !seconds.is_finite() || !state.iter().all(|x| x.is_finite()) {
        return Err(Dop853Error::NumericalBreakdown);
    }
    *evaluations += 1;
    let derivative = f(seconds, state)?;
    if !derivative.iter().all(|x| x.is_finite()) {
        return Err(Dop853Error::NonFiniteDerivative);
    }
    Ok(derivative)
}

fn combine<const N: usize>(
    start: &[f64; N],
    h: f64,
    weights: &[f64],
    stages: &[[f64; N]; 16],
) -> [f64; N] {
    let mut next = *start;
    for (stage, weight) in weights.iter().enumerate() {
        if *weight == 0.0 {
            continue;
        }
        for component in 0..N {
            next[component] += h * *weight * stages[stage][component];
        }
    }
    next
}

fn error_norm<const N: usize>(
    start: &[f64; N],
    next: &[f64; N],
    stages: &[[f64; N]; 16],
    h: f64,
    config: &Dop853Config<N>,
) -> f64 {
    let mut norm5 = 0.0;
    let mut norm3 = 0.0;
    for component in 0..N {
        let mut error5 = 0.0;
        let mut error3 = 0.0;
        for stage in 0..13 {
            error5 += E5[stage] * stages[stage][component];
            error3 += E3[stage] * stages[stage][component];
        }
        let scale = config.absolute_tolerances[component]
            + config.relative_tolerance * start[component].abs().max(next[component].abs());
        norm5 += (error5 / scale).powi(2);
        norm3 += (error3 / scale).powi(2);
    }
    if norm5 == 0.0 && norm3 == 0.0 {
        return 0.0;
    }
    let denominator = norm5 + 0.01 * norm3;
    h.abs() * norm5 / (denominator * N as f64).sqrt()
}

fn dense_segment<const N: usize, F, C>(
    f: &mut F,
    cancelled: &mut C,
    seconds: f64,
    start: &[f64; N],
    next: &[f64; N],
    h: f64,
    stages: &mut [[f64; N]; 16],
    evaluations: &mut usize,
    budget: usize,
) -> Result<Dop853DenseSegment<N>, Dop853Error>
where
    F: FnMut(f64, &[f64; N]) -> Result<[f64; N], Dop853Error>,
    C: FnMut() -> bool,
{
    for stage in 13..16 {
        let trial = combine(start, h, &A[stage][..stage], stages);
        stages[stage] = evaluate(
            f,
            cancelled,
            seconds + C[stage] * h,
            &trial,
            evaluations,
            budget,
        )?;
    }
    let mut coefficients = [[0.0; N]; 7];
    for component in 0..N {
        let delta = next[component] - start[component];
        coefficients[0][component] = delta;
        coefficients[1][component] = h * stages[0][component] - delta;
        coefficients[2][component] =
            2.0 * delta - h * (stages[12][component] + stages[0][component]);
        for polynomial in 0..4 {
            let mut value = 0.0;
            for stage in 0..16 {
                value += D[polynomial][stage] * stages[stage][component];
            }
            coefficients[polynomial + 3][component] = h * value;
        }
    }
    if !coefficients.iter().flatten().all(|x| x.is_finite()) {
        return Err(Dop853Error::NumericalBreakdown);
    }
    Ok(Dop853DenseSegment {
        start_seconds: seconds,
        end_seconds: seconds + h,
        valid_until_seconds: seconds + h,
        start_state: *start,
        coefficients,
    })
}

pub fn solve_dop853<const N: usize, F, C>(
    initial: [f64; N],
    end_seconds: f64,
    config: Dop853Config<N>,
    derivative: F,
    cancelled: C,
) -> Result<Dop853Result<N>, Dop853Error>
where
    F: FnMut(f64, &[f64; N]) -> Result<[f64; N], Dop853Error>,
    C: FnMut() -> bool,
{
    solve_dop853_with_event(initial, end_seconds, config, derivative, cancelled, |_| {
        Ok(None)
    })
}

pub fn solve_dop853_with_event<const N: usize, F, C, O>(
    initial: [f64; N],
    end_seconds: f64,
    config: Dop853Config<N>,
    mut derivative: F,
    mut cancelled: C,
    mut observe: O,
) -> Result<Dop853Result<N>, Dop853Error>
where
    F: FnMut(f64, &[f64; N]) -> Result<[f64; N], Dop853Error>,
    C: FnMut() -> bool,
    O: FnMut(&Dop853DenseSegment<N>) -> Result<Option<f64>, Dop853Error>,
{
    config.validate()?;
    if !end_seconds.is_finite() || end_seconds <= 0.0 || !initial.iter().all(|x| x.is_finite()) {
        return Err(Dop853Error::InvalidInitialState);
    }
    let mut result = Dop853Result {
        final_state: initial,
        reached_seconds: 0.0,
        segments: Vec::new(),
        accepted_steps: 0,
        rejected_steps: 0,
        evaluations: 0,
        event_seconds: None,
    };
    let mut state = initial;
    let mut seconds = 0.0;
    let mut step = config.initial_step_s;
    let mut previous_rejected = false;
    let mut initial_derivative = evaluate(
        &mut derivative,
        &mut cancelled,
        seconds,
        &state,
        &mut result.evaluations,
        config.maximum_evaluations,
    )?;
    for _ in 0..config.maximum_attempts {
        if seconds >= end_seconds {
            return Ok(result);
        }
        let h = step.min(config.maximum_step_s).min(end_seconds - seconds);
        if h <= 0.0 || seconds + h == seconds {
            return Err(Dop853Error::StepUnderflow);
        }
        let mut stages = [[0.0; N]; 16];
        stages[0] = initial_derivative;
        for stage in 1..12 {
            let trial = combine(&state, h, &A[stage][..stage], &stages);
            stages[stage] = evaluate(
                &mut derivative,
                &mut cancelled,
                seconds + C[stage] * h,
                &trial,
                &mut result.evaluations,
                config.maximum_evaluations,
            )?;
        }
        let next = combine(&state, h, &B, &stages);
        stages[12] = evaluate(
            &mut derivative,
            &mut cancelled,
            seconds + h,
            &next,
            &mut result.evaluations,
            config.maximum_evaluations,
        )?;
        let error = error_norm(&state, &next, &stages, h, &config);
        if !error.is_finite() {
            return Err(Dop853Error::NumericalBreakdown);
        }
        let factor = if error == 0.0 {
            10.0
        } else {
            (0.9 * error.powf(-1.0 / 8.0)).clamp(0.2, 10.0)
        };
        if error <= 1.0 {
            if result.accepted_steps >= config.maximum_segments {
                return Err(Dop853Error::BudgetExceeded);
            }
            let segment = dense_segment(
                &mut derivative,
                &mut cancelled,
                seconds,
                &state,
                &next,
                h,
                &mut stages,
                &mut result.evaluations,
                config.maximum_evaluations,
            )?;
            seconds += h;
            state = next;
            initial_derivative = stages[12];
            result.accepted_steps += 1;
            result.final_state = state;
            result.reached_seconds = seconds;
            let event = observe(&segment)?;
            if let Some(time) = event {
                if !time.is_finite() || time < segment.start_seconds || time > segment.end_seconds {
                    return Err(Dop853Error::NumericalBreakdown);
                }
                result.final_state = segment
                    .evaluate(time)
                    .ok_or(Dop853Error::NumericalBreakdown)?;
                result.reached_seconds = time;
                result.event_seconds = Some(time);
                let mut segment = segment;
                segment.valid_until_seconds = time;
                result.segments.push(segment);
                return Ok(result);
            }
            result.segments.push(segment);
            step = (h * if previous_rejected {
                factor.min(1.0)
            } else {
                factor
            })
            .min(config.maximum_step_s);
            previous_rejected = false;
            if seconds >= end_seconds {
                return Ok(result);
            }
        } else {
            result.rejected_steps += 1;
            step = h * factor.min(1.0);
            previous_rejected = true;
            if step < config.minimum_step_s {
                return Err(Dop853Error::StepUnderflow);
            }
        }
    }
    Err(Dop853Error::BudgetExceeded)
}
