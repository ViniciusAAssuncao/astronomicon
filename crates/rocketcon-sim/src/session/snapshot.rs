use rocketcon_app::aeroespacial::VehicleTickReport;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlightEventKind {
    AtmosphericEntry,
    AtmosphericExit,
    SurfaceContact,
    Liftoff,
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
        events
    }
}

#[cfg(test)]
mod tests {
    use super::{FlightEvent, FlightEventKind, FlightSnapshot};
    use uuid::Uuid;

    fn snapshot(epoch: f64) -> FlightSnapshot {
        FlightSnapshot {
            vehicle_id: Uuid::nil(),
            reference_body_id: Uuid::nil(),
            total_epoch_seconds: epoch,
            position_m: [0.0; 3],
            reference_body_position_m: None,
            reference_body_radius_m: None,
            velocity_m_s: [0.0; 3],
            angular_velocity_rad_s: [0.0; 3],
            speed_m_s: 0.0,
            reference_speed_m_s: None,
            altitude_m: None,
            mach: None,
            dynamic_pressure_pa: None,
            total_g_load: Some(0.0),
            surface_contact: Some(false),
        }
    }

    #[test]
    fn records_only_observed_transitions_at_current_epoch() {
        let previous = snapshot(4.0);
        let mut current = snapshot(4.02);
        current.mach = Some(0.8);
        current.surface_contact = Some(true);
        let events = FlightEvent::between(&previous, &current);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].kind, FlightEventKind::AtmosphericEntry);
        assert_eq!(events[1].kind, FlightEventKind::SurfaceContact);
        assert!(events.iter().all(|event| event.total_epoch_seconds == 4.02));
        assert!(FlightEvent::between(&current, &current).is_empty());
    }
}
