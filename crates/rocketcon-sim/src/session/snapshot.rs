use rocketcon_app::aeroespacial::VehicleTickReport;
use rocketcon_core::domain::VehiclePhysicalState;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub struct FlightSnapshot {
    pub vehicle_id: Uuid,
    pub reference_body_id: Uuid,
    pub total_epoch_seconds: f64,
    pub position_m: [f64; 3],
    pub velocity_m_s: [f64; 3],
    pub speed_m_s: f64,
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
        Self {
            vehicle_id: state.vehicle_id(),
            reference_body_id: state.reference_body_id(),
            total_epoch_seconds: state.captured_total_epoch().value(),
            position_m: [position.0, position.1, position.2],
            velocity_m_s: [velocity.0, velocity.1, velocity.2],
            speed_m_s: state.speed().value(),
            altitude_m: None,
            mach: None,
            dynamic_pressure_pa: None,
            total_g_load: None,
            surface_contact: None,
        }
    }

    pub fn from_report(report: &VehicleTickReport) -> Self {
        let mut snapshot = Self::from_state(report.physical_state());
        snapshot.altitude_m = report.aerodynamics().map(|a| a.altitude.value());
        snapshot.mach = report.mach_number();
        snapshot.dynamic_pressure_pa = report.dynamic_pressure().map(|q| q.value());
        snapshot.total_g_load = Some(report.total_g_load());
        snapshot.surface_contact = Some(report.has_contact());
        snapshot
    }
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
