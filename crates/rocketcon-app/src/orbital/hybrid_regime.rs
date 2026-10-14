use crate::error::{RocketError, RocketResult};
use astronomicon_app::ephemeris::EphemerisCatalog;
use astronomicon_core::units::constants::GRAVITATIONAL_CONSTANT;
use astronomicon_core::units::{Duration, Vector3};
use rocketcon_core::domain::VehiclePhysicalState;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GravityRegime {
    Kepler,
    Specialized,
    Restricted { sources: Vec<Uuid> },
}

impl GravityRegime {
    pub fn sources(&self) -> Option<&[Uuid]> {
        match self {
            Self::Kepler | Self::Specialized => None,
            Self::Restricted { sources } => Some(sources),
        }
    }

    pub(crate) fn label(&self, total_sources: usize) -> &'static str {
        match self {
            Self::Kepler => "kepler",
            Self::Specialized => "specialized_forces",
            Self::Restricted { sources } if sources.len() == total_sources => "nbody",
            Self::Restricted { sources } if sources.len() == 2 => "three_body",
            Self::Restricted { .. } => "nbody_selected",
        }
    }
}

#[derive(Clone, Copy)]
pub struct RegimePolicy {
    pub position_error_budget_m: f64,
    pub orbital_lookahead_fraction: f64,
}

impl RegimePolicy {
    pub fn flight() -> Self {
        Self {
            position_error_budget_m: 10.0,
            orbital_lookahead_fraction: 0.125,
        }
    }

    pub fn forecast() -> Self {
        Self {
            position_error_budget_m: 10.0,
            orbital_lookahead_fraction: 0.125,
        }
    }

    fn validate(self) -> RocketResult<()> {
        if !self.position_error_budget_m.is_finite()
            || self.position_error_budget_m <= 0.0
            || !self.orbital_lookahead_fraction.is_finite()
            || self.orbital_lookahead_fraction <= 0.0
            || self.orbital_lookahead_fraction > 1.0
        {
            return Err(RocketError::Generic(
                "invalid hybrid orbital regime policy".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct Body {
    id: Uuid,
    mu: f64,
}

fn acceleration(mu: f64, displacement: Vector3) -> Option<Vector3> {
    let r2 = displacement.dot(&displacement);
    if !r2.is_finite() || r2 <= 0.0 {
        return None;
    }
    let factor = mu / (r2 * r2.sqrt());
    if !factor.is_finite() {
        return None;
    }
    Some(displacement * factor)
}

fn displaced_acceleration(
    source: Vector3,
    craft: Vector3,
    reference: Vector3,
    mu: f64,
) -> Option<(f64, f64)> {
    let direct = acceleration(mu, source - craft)?;
    let reference_acc = acceleration(mu, source - reference)?;
    Some(((direct - reference_acc).magnitude(), direct.magnitude()))
}

fn source_positions(
    catalog: &EphemerisCatalog,
    epoch: Duration,
    elapsed: f64,
    ids: &[Uuid],
) -> RocketResult<HashMap<Uuid, Vector3>> {
    let positions = catalog
        .positions_for_ids_at_offset(epoch, elapsed, ids)
        .map_err(|error| {
            RocketError::Generic(format!("hybrid ephemeris evaluation failed: {error}"))
        })?;
    Ok(positions.into_iter().map(|(id, p)| (id, p.raw())).collect())
}

pub fn choose_regime(
    catalog: &EphemerisCatalog,
    state: &VehiclePhysicalState,
    dt: Duration,
    powered: bool,
    policy: RegimePolicy,
) -> RocketResult<GravityRegime> {
    choose_regime_at_offset(
        catalog,
        state.captured_total_epoch(),
        0.0,
        state.reference_body_id(),
        state.position().raw(),
        state.velocity().raw(),
        dt,
        powered,
        policy,
    )
}

pub fn choose_regime_at_offset(
    catalog: &EphemerisCatalog,
    epoch: Duration,
    offset_seconds: f64,
    reference_id: Uuid,
    craft: Vector3,
    velocity: Vector3,
    dt: Duration,
    powered: bool,
    policy: RegimePolicy,
) -> RocketResult<GravityRegime> {
    policy.validate()?;
    if !epoch.value().is_finite() || !offset_seconds.is_finite() || offset_seconds < 0.0 {
        return Err(RocketError::Generic("invalid hybrid epoch offset".into()));
    }
    if !dt.value().is_finite() || dt.value() <= 0.0 {
        return Err(RocketError::Generic("invalid hybrid tick duration".into()));
    }
    let mut bodies = catalog
        .gravitating_bodies()
        .into_iter()
        .map(|(id, mass)| Body {
            id,
            mu: mass.value() * GRAVITATIONAL_CONSTANT,
        })
        .collect::<Vec<_>>();
    bodies.sort_by_key(|body| body.id);
    if bodies.is_empty()
        || bodies
            .iter()
            .any(|body| !body.mu.is_finite() || body.mu <= 0.0)
    {
        return Err(RocketError::Generic(
            "invalid hybrid gravitational sources".into(),
        ));
    }
    let reference = bodies
        .iter()
        .find(|body| body.id == reference_id)
        .ok_or_else(|| RocketError::Generic("reference is not a massive catalog body".into()))?;
    let ids = bodies.iter().map(|body| body.id).collect::<Vec<_>>();
    if bodies.len() == 1 && !powered {
        return Ok(GravityRegime::Kepler);
    }
    if bodies.len() <= 2 && powered {
        return Ok(GravityRegime::Restricted { sources: ids });
    }
    let positions_initial = source_positions(catalog, epoch, offset_seconds, &ids)?;
    let ref_pos = *positions_initial
        .get(&reference_id)
        .ok_or_else(|| RocketError::Generic("reference ephemeris missing".into()))?;
    let separation = (craft - ref_pos).magnitude();
    if !separation.is_finite()
        || separation <= 0.0
        || ![velocity.0, velocity.1, velocity.2]
            .iter()
            .all(|v| v.is_finite())
    {
        return Err(RocketError::Generic("invalid hybrid orbital state".into()));
    }
    let dynamical_time = (separation * (separation / reference.mu).sqrt()).abs();
    if !dynamical_time.is_finite() || dynamical_time <= 0.0 {
        return Err(RocketError::Generic(
            "invalid hybrid orbital timescale".into(),
        ));
    }
    let horizon = (dynamical_time * policy.orbital_lookahead_fraction).max(dt.value());
    if !horizon.is_finite() {
        return Err(RocketError::Generic(
            "invalid hybrid lookahead horizon".into(),
        ));
    }
    let sample_times = [0.0, horizon * 0.5, horizon];
    let mut maximum_tidal = vec![0.0_f64; bodies.len()];
    let mut maximum_direct = vec![0.0_f64; bodies.len()];
    for (sample_idx, t) in sample_times.into_iter().enumerate() {
        let positions = if sample_idx == 0 {
            positions_initial.clone()
        } else {
            source_positions(catalog, epoch, offset_seconds + t, &ids)?
        };
        let reference_at_t = *positions
            .get(&reference_id)
            .ok_or_else(|| RocketError::Generic("reference ephemeris missing".into()))?;
        let estimated_craft = craft + velocity * t;
        for (index, body) in bodies.iter().enumerate() {
            if body.id == reference_id {
                continue;
            }
            let source = *positions
                .get(&body.id)
                .ok_or_else(|| RocketError::Generic("source ephemeris missing".into()))?;
            let Some((tidal, direct)) =
                displaced_acceleration(source, estimated_craft, reference_at_t, body.mu)
            else {
                return Ok(GravityRegime::Restricted { sources: ids });
            };
            maximum_tidal[index] = maximum_tidal[index].max(tidal);
            maximum_direct[index] = maximum_direct[index].max(direct);
        }
    }
    let budget_acceleration = policy.position_error_budget_m * 2.0 / horizon.powi(2);
    if !budget_acceleration.is_finite() || budget_acceleration <= 0.0 {
        return Ok(GravityRegime::Restricted { sources: ids });
    }
    let tidal_sum: f64 = maximum_tidal.iter().sum();
    if !powered && tidal_sum <= budget_acceleration && bodies.len() >= 1 {
        return Ok(GravityRegime::Kepler);
    }
    let others = bodies
        .iter()
        .enumerate()
        .filter(|(_, body)| body.id != reference_id)
        .map(|(index, body)| (body.id, maximum_direct[index]))
        .collect::<Vec<_>>();
    let sources = select_sources(reference_id, others, budget_acceleration);
    Ok(GravityRegime::Restricted { sources })
}

fn select_sources(reference: Uuid, mut others: Vec<(Uuid, f64)>, budget: f64) -> Vec<Uuid> {
    others.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let mut selected = vec![reference];
    let mut omitted: f64 = others.iter().map(|(_, acceleration)| acceleration).sum();
    for (id, magnitude) in others {
        if selected.len() >= 2 && omitted <= budget {
            break;
        }
        selected.push(id);
        omitted -= magnitude;
    }
    selected.sort();
    selected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_tide_and_absolute_force_are_distinct() {
        let source = Vector3::new(1.0e9, 0.0, 0.0);
        let craft = Vector3::new(1.0e6, 0.0, 0.0);
        let reference = craft;
        let (tide, direct) = displaced_acceleration(source, craft, reference, 1.0e20).unwrap();
        assert!(direct > 0.0);
        assert!(tide.abs() < 1e-12);
    }

    #[test]
    fn selected_sources_respect_omitted_acceleration_budget() {
        let reference = Uuid::from_u128(1);
        let one = Uuid::from_u128(2);
        let two = Uuid::from_u128(3);
        let three = Uuid::from_u128(4);
        let others = vec![(three, 0.001), (two, 10.0), (one, 100.0)];
        assert_eq!(
            select_sources(reference, others.clone(), 0.01),
            vec![reference, one, two]
        );
        assert_eq!(
            select_sources(reference, others, 0.0001),
            vec![reference, one, two, three]
        );
    }

    #[test]
    fn policy_rejects_nonfinite_and_nonpositive_bounds() {
        for budget in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(
                RegimePolicy {
                    position_error_budget_m: budget,
                    orbital_lookahead_fraction: 0.125
                }
                .validate()
                .is_err()
            );
        }
    }
}
