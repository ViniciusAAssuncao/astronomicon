use rocketcon_app::aeroespacial::VehicleTickReport;
use astronomicon_core::units::{Quaternion, Vector3};
use rocketcon_core::domain::VehiclePhysicalState;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub struct FlightSnapshot {
    pub vehicle_id: Uuid,
    pub reference_body_id: Uuid,
    pub total_epoch_seconds: f64,
    pub position_m: [f64; 3],
    pub reference_body_position_m: Option<[f64; 3]>,
    pub reference_body_radius_m: Option<f64>,
    pub velocity_m_s: [f64; 3],
    pub angular_velocity_rad_s: [f64; 3],
    pub speed_m_s: f64,
    pub reference_speed_m_s: Option<f64>,
    pub reference_vertical_speed_m_s: Option<f64>,
    pub reference_horizontal_speed_m_s: Option<f64>,
    pub local_up_body: Option<[f64; 3]>,
    pub main_engine_loads: Vec<(String, f64)>,
    pub battery_stored_j: Option<f64>,
    pub battery_capacity_j: Option<f64>,
    pub fuel_reserves: Vec<(String, f64, f64)>,
    pub altitude_m: Option<f64>,
    pub mach: Option<f64>,
    pub dynamic_pressure_pa: Option<f64>,
    pub total_g_load: Option<f64>,
    pub surface_contact: Option<bool>,
}

impl FlightSnapshot {
    pub fn from_state(state: &VehiclePhysicalState) -> Self {
        let position = state.position().raw();
        let velocity = state.velocity().raw();
        let angular_velocity = state.angular_velocity().raw();
        Self {
            vehicle_id: state.vehicle_id(),
            reference_body_id: state.reference_body_id(),
            total_epoch_seconds: state.captured_total_epoch().value(),
            position_m: [position.0, position.1, position.2],
            reference_body_position_m: None,
            reference_body_radius_m: None,
            velocity_m_s: [velocity.0, velocity.1, velocity.2],
            angular_velocity_rad_s: [angular_velocity.0, angular_velocity.1, angular_velocity.2],
            speed_m_s: state.speed().value(),
            reference_speed_m_s: None,
            reference_vertical_speed_m_s: None,
            reference_horizontal_speed_m_s: None,
            local_up_body: None,
            main_engine_loads: Vec::new(),
            battery_stored_j: None,
            battery_capacity_j: None,
            fuel_reserves: Vec::new(),
            altitude_m: None,
            mach: None,
            dynamic_pressure_pa: None,
            total_g_load: None,
            surface_contact: None,
        }
    }

    pub fn from_report(report: &VehicleTickReport) -> Self {
        let mut snapshot = Self::from_state(report.physical_state());
        snapshot.reference_body_position_m = Some(report.reference_body_position_m);
        snapshot.reference_body_radius_m = Some(report.reference_body_radius_m);
        snapshot.reference_speed_m_s = relative_speed(
            snapshot.velocity_m_s,
            report.reference_body_velocity_m_s,
        );
        snapshot.reference_vertical_speed_m_s = reference_vertical_speed(
            snapshot.position_m,
            report.reference_body_position_m,
            snapshot.velocity_m_s,
            report.reference_body_velocity_m_s,
        );
        snapshot.reference_horizontal_speed_m_s = reference_horizontal_speed(
            snapshot.reference_speed_m_s,
            snapshot.reference_vertical_speed_m_s,
        );
        snapshot.local_up_body = local_up_body(
            snapshot.position_m,
            report.reference_body_position_m,
            report.physical_state().orientation(),
        );
        snapshot.main_engine_loads = report.main_engine_loads.iter()
            .map(|(id, load)| (id.to_string(), *load)).collect();
        snapshot.battery_stored_j = Some(report.power_budget.total_stored_energy.value());
        snapshot.battery_capacity_j = Some(report.power_budget.total_battery_capacity.value());
        snapshot.altitude_m = report.aerodynamics().map(|a| a.altitude.value()).or_else(|| {
            geometric_altitude(
                snapshot.position_m,
                report.reference_body_position_m,
                report.reference_body_radius_m,
            )
        });
        snapshot.mach = report.mach_number();
        snapshot.dynamic_pressure_pa = report.dynamic_pressure().map(|q| q.value());
        snapshot.total_g_load = Some(report.total_g_load());
        snapshot.surface_contact = Some(report.has_contact());
        snapshot
    }
}

pub(crate) fn geometric_altitude(position: [f64; 3], body: [f64; 3], radius: f64) -> Option<f64> {
    if !position.iter().chain(body.iter()).all(|v| v.is_finite()) || !radius.is_finite() || radius <= 0.0 {
        return None;
    }
    let delta = [position[0] - body[0], position[1] - body[1], position[2] - body[2]];
    Some(delta[0].hypot(delta[1]).hypot(delta[2]) - radius)
}

pub(crate) fn relative_speed(velocity: [f64; 3], body_velocity: [f64; 3]) -> Option<f64> {
    if !velocity.iter().chain(body_velocity.iter()).all(|v| v.is_finite()) {
        return None;
    }
    let delta = [
        velocity[0] - body_velocity[0],
        velocity[1] - body_velocity[1],
        velocity[2] - body_velocity[2],
    ];
    Some(delta[0].hypot(delta[1]).hypot(delta[2]))
}

pub(crate) fn reference_vertical_speed(
    position: [f64; 3],
    body_position: [f64; 3],
    velocity: [f64; 3],
    body_velocity: [f64; 3],
) -> Option<f64> {
    if !position.iter().chain(body_position.iter()).chain(velocity.iter())
        .chain(body_velocity.iter()).all(|value| value.is_finite()) {
        return None;
    }
    let radial = [
        position[0] - body_position[0],
        position[1] - body_position[1],
        position[2] - body_position[2],
    ];
    let distance = radial[0].hypot(radial[1]).hypot(radial[2]);
    if distance <= 0.0 || !distance.is_finite() {
        return None;
    }
    let relative_velocity = [
        velocity[0] - body_velocity[0],
        velocity[1] - body_velocity[1],
        velocity[2] - body_velocity[2],
    ];
    let projection = radial.iter().zip(relative_velocity.iter())
        .map(|(radius, speed)| radius * speed / distance)
        .sum::<f64>();
    projection.is_finite().then_some(projection)
}

pub(crate) fn reference_horizontal_speed(
    reference_speed: Option<f64>,
    vertical_speed: Option<f64>,
) -> Option<f64> {
    let speed = reference_speed?;
    let vertical = vertical_speed?;
    if !speed.is_finite() || !vertical.is_finite() || speed < 0.0 {
        return None;
    }
    Some(((speed - vertical.abs()).max(0.0) * (speed + vertical.abs())).sqrt())
}

pub(crate) fn local_up_body(
    position: [f64; 3],
    body_position: [f64; 3],
    orientation: Quaternion,
) -> Option<[f64; 3]> {
    if !position.iter().chain(body_position.iter()).all(|value| value.is_finite()) {
        return None;
    }
    let radial = Vector3::new(
        position[0] - body_position[0],
        position[1] - body_position[1],
        position[2] - body_position[2],
    );
    let distance = radial.magnitude();
    if distance <= 0.0 || !distance.is_finite() {
        return None;
    }
    let up = orientation.inverse().rotate_vector(radial / distance).normalized();
    let result = [up.0, up.1, up.2];
    result.iter().all(|value| value.is_finite()).then_some(result)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlightEventKind {
    AtmosphericEntry,
    AtmosphericExit,
    SurfaceContact,
    Liftoff,
    EngineIgnition,
    EngineCutoff,
    EngineThrottle,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlightEvent {
    pub kind: FlightEventKind,
    pub total_epoch_seconds: f64,
}

impl FlightEvent {
    pub fn between(previous: &FlightSnapshot, current: &FlightSnapshot) -> Vec<Self> {
        let mut events = Vec::new();
        let atmosphere = match (previous.mach.is_some(), current.mach.is_some()) {
            (true, false) => Some(FlightEventKind::AtmosphericExit),
            (false, true) if previous.total_g_load.is_some() => {
                Some(FlightEventKind::AtmosphericEntry)
            }
            _ => None,
        };
        if let Some(kind) = atmosphere {
            events.push(Self {
                kind,
                total_epoch_seconds: current.total_epoch_seconds,
            });
        }
        let contact = match (previous.surface_contact, current.surface_contact) {
            (Some(false), Some(true)) => Some(FlightEventKind::SurfaceContact),
            (Some(true), Some(false)) => Some(FlightEventKind::Liftoff),
            _ => None,
        };
        if let Some(kind) = contact {
            events.push(Self {
                kind,
                total_epoch_seconds: current.total_epoch_seconds,
            });
        }
        for (id, previous_load) in &previous.main_engine_loads {
            if *previous_load > 0.0 && current.main_engine_loads.iter()
                .any(|(current_id, load)| current_id == id && *load == 0.0) {
                events.push(Self { kind: FlightEventKind::EngineCutoff,
                    total_epoch_seconds: current.total_epoch_seconds });
            }
        }
        events
    }
}