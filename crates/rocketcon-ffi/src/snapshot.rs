use crate::ffi::{FfiVec3, FlightSnapshot};
use rocketcon_sim::FlightSnapshot as SessionSnapshot;

impl From<&SessionSnapshot> for FlightSnapshot {
    fn from(snapshot: &SessionSnapshot) -> Self {
        Self {
            total_epoch_seconds: snapshot.total_epoch_seconds,
            position_m: snapshot.position_m.into(),
            has_reference_body: snapshot.reference_body_position_m.is_some() && snapshot.reference_body_radius_m.is_some(),
            reference_body_position_m: snapshot.reference_body_position_m.unwrap_or_default().into(),
            reference_body_radius_m: snapshot.reference_body_radius_m.unwrap_or_default(),
            velocity_m_s: snapshot.velocity_m_s.into(),
            angular_velocity_rad_s: snapshot.angular_velocity_rad_s.into(),
            speed_m_s: snapshot.speed_m_s,
            has_reference_speed: snapshot.reference_speed_m_s.is_some(),
            reference_speed_m_s: snapshot.reference_speed_m_s.unwrap_or_default(),
            has_altitude: snapshot.altitude_m.is_some(),
            altitude_m: snapshot.altitude_m.unwrap_or_default(),
            has_mach: snapshot.mach.is_some(),
            mach: snapshot.mach.unwrap_or_default(),
            has_dynamic_pressure: snapshot.dynamic_pressure_pa.is_some(),
            dynamic_pressure_pa: snapshot.dynamic_pressure_pa.unwrap_or_default(),
            has_g_load: snapshot.total_g_load.is_some(),
            total_g_load: snapshot.total_g_load.unwrap_or_default(),
            has_surface_contact: snapshot.surface_contact.is_some(),
            surface_contact: snapshot.surface_contact.unwrap_or_default(),
        }
    }
}

impl From<[f64; 3]> for FfiVec3 {
    fn from(value: [f64; 3]) -> Self {
        Self {
            x: value[0],
            y: value[1],
            z: value[2],
        }
    }
}
