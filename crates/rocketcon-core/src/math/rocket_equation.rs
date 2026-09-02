use crate::domain::{ComponentDetails, ComponentRecord, EngineState, VehicleComponentEntry};
use astronomicon_core::units::{Acceleration, Duration, Force, Mass, MassRate, Speed, Vector3};
use std::collections::HashMap;
use uuid::Uuid;

pub fn tsiolkovsky_delta_v(
    effective_exhaust_velocity: Speed,
    wet_mass: Mass,
    dry_mass: Mass,
) -> Speed {
    let ve = effective_exhaust_velocity.value();
    let m_wet = wet_mass.value();
    let m_dry = dry_mass.value();

    if ve <= 0.0
        || m_wet <= 0.0
        || m_dry <= 0.0
        || m_wet < m_dry
        || !ve.is_finite()
        || !m_wet.is_finite()
        || !m_dry.is_finite()
    {
        return Speed::new(0.0);
    }

    let dv = ve * (m_wet / m_dry).ln();
    if !dv.is_finite() || dv < 0.0 {
        Speed::new(0.0)
    } else {
        Speed::new(dv)
    }
}

pub fn mass_ratio(wet_mass: Mass, dry_mass: Mass) -> f64 {
    let m_wet = wet_mass.value();
    let m_dry = dry_mass.value();

    if m_wet <= 0.0 || m_dry <= 0.0 || !m_wet.is_finite() || !m_dry.is_finite() {
        return 0.0;
    }

    let ratio = m_wet / m_dry;
    if !ratio.is_finite() || ratio < 0.0 {
        0.0
    } else {
        ratio
    }
}

pub fn propellant_mass_fraction(wet_mass: Mass, dry_mass: Mass) -> f64 {
    let m_wet = wet_mass.value();
    let m_dry = dry_mass.value();

    if m_wet <= 0.0 || m_dry <= 0.0 || m_wet < m_dry || !m_wet.is_finite() || !m_dry.is_finite() {
        return 0.0;
    }

    let frac = (m_wet - m_dry) / m_wet;
    if !frac.is_finite() || frac < 0.0 {
        0.0
    } else {
        frac.clamp(0.0, 1.0)
    }
}

pub fn combined_effective_exhaust_velocity(
    total_thrust: Force,
    total_mass_flow_rate: MassRate,
) -> Speed {
    let f = total_thrust.value();
    let m_dot = total_mass_flow_rate.value();

    if f <= 0.0 || m_dot <= 0.0 || !f.is_finite() || !m_dot.is_finite() {
        return Speed::new(0.0);
    }

    let ve = f / m_dot;
    if !ve.is_finite() || ve < 0.0 {
        Speed::new(0.0)
    } else {
        Speed::new(ve)
    }
}

pub fn burn_time(propellant_mass: Mass, mass_flow_rate: MassRate) -> Duration {
    let m_prop = propellant_mass.value();
    let m_dot = mass_flow_rate.value();

    if m_prop <= 0.0 || m_dot <= 0.0 || !m_prop.is_finite() || !m_dot.is_finite() {
        return Duration::new(0.0);
    }

    let t = m_prop / m_dot;
    if !t.is_finite() || t < 0.0 {
        Duration::new(0.0)
    } else {
        Duration::new(t)
    }
}

pub fn thrust_to_weight_ratio(
    total_thrust: Force,
    mass: Mass,
    local_gravity: Acceleration,
) -> f64 {
    let f = total_thrust.value();
    let m = mass.value();
    let g = local_gravity.value();

    if f <= 0.0 || m <= 0.0 || g <= 0.0 || !f.is_finite() || !m.is_finite() || !g.is_finite() {
        return 0.0;
    }

    let weight = m * g;
    if weight <= 0.0 || !weight.is_finite() {
        return 0.0;
    }

    let twr = f / weight;
    if !twr.is_finite() || twr < 0.0 {
        0.0
    } else {
        twr
    }
}

pub fn combined_thrust_vector(
    entries: &[(VehicleComponentEntry, ComponentRecord)],
    active_stages: &[u32],
    engine_states: &HashMap<Uuid, EngineState>,
) -> Vector3 {
    let mut total_thrust = Vector3::zero();

    for (entry, record) in entries {
        if !active_stages.contains(&entry.stage_index()) {
            continue;
        }

        if let ComponentDetails::Engine(engine) = record.details() {
            let state = engine_states
                .get(&entry.id())
                .or_else(|| engine_states.get(&entry.component_id()));

            if let Some(state) = state {
                if matches!(state, EngineState::MainStage | EngineState::Throttling) {
                    let thrust = engine.max_thrust().value();
                    if thrust.is_finite() && thrust > 0.0 {
                        let dir = entry
                            .actuation_axis()
                            .unwrap_or(Vector3::new(0.0, 0.0, 1.0));
                        total_thrust = total_thrust + dir * thrust;
                    }
                }
            }
        }
    }

    total_thrust
}