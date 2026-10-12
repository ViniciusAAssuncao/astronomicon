use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EngineDemand {
    pub id: Uuid,
    pub fuel_id: Uuid,
    pub fuel_flow_kg_s: f64,
    pub oxidizer: Option<(Uuid, f64)>,
    pub load: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TankReserve {
    pub id: Uuid,
    pub propellant_id: Uuid,
    pub capacity_kg: f64,
    pub stored_kg: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PropellantBudget {
    pub effective_loads: HashMap<Uuid, f64>,
    pub tanks: Vec<TankReserve>,
    pub depleted_engines: Vec<Uuid>,
}

pub fn plan_propellant_budget(
    engines: &[EngineDemand], tanks: &[TankReserve], dt_seconds: f64,
) -> PropellantBudget {
    let mut available = HashMap::<Uuid, f64>::new();
    for tank in tanks {
        *available.entry(tank.propellant_id).or_default() += tank.stored_kg;
    }
    let mut requested = HashMap::<Uuid, f64>::new();
    for engine in engines {
        if engine.load <= 0.0 { continue; }
        *requested.entry(engine.fuel_id).or_default() +=
            engine.fuel_flow_kg_s * engine.load * dt_seconds;
        if let Some((id, flow)) = engine.oxidizer {
            *requested.entry(id).or_default() += flow * engine.load * dt_seconds;
        }
    }
    let mut factors = HashMap::<Uuid, f64>::new();
    for (id, demand) in requested {
        if demand > 0.0 {
            factors.insert(id, (available.get(&id).copied().unwrap_or(0.0) / demand).clamp(0.0, 1.0));
        }
    }
    let mut effective_loads = HashMap::new();
    let mut consumption = HashMap::<Uuid, f64>::new();
    for engine in engines {
        let mut factor = factors.get(&engine.fuel_id).copied().unwrap_or(0.0);
        if let Some((id, _)) = engine.oxidizer {
            factor = factor.min(factors.get(&id).copied().unwrap_or(0.0));
        }
        let load = engine.load * factor;
        effective_loads.insert(engine.id, load);
        *consumption.entry(engine.fuel_id).or_default() +=
            engine.fuel_flow_kg_s * load * dt_seconds;
        if let Some((id, flow)) = engine.oxidizer {
            *consumption.entry(id).or_default() += flow * load * dt_seconds;
        }
    }
    let tanks: Vec<TankReserve> = tanks.iter().map(|tank| {
        let total = available.get(&tank.propellant_id).copied().unwrap_or(0.0);
        let spent = consumption.get(&tank.propellant_id).copied().unwrap_or(0.0);
        let share = if total > 0.0 { tank.stored_kg / total } else { 0.0 };
        TankReserve { stored_kg: (tank.stored_kg - spent * share).max(0.0), ..*tank }
    }).collect();
    let mut remaining = HashMap::<Uuid, f64>::new();
    for tank in &tanks {
        *remaining.entry(tank.propellant_id).or_default() += tank.stored_kg;
    }
    let depleted_engines = engines.iter().filter(|engine| engine.load > 0.0 &&
        (remaining.get(&engine.fuel_id).copied().unwrap_or(0.0) <= 1e-9 ||
            engine.oxidizer.map_or(false, |(id, _)|
                remaining.get(&id).copied().unwrap_or(0.0) <= 1e-9)))
        .map(|engine| engine.id).collect();
    PropellantBudget { effective_loads, tanks, depleted_engines }
}