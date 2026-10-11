use crate::ffi::{FfiVec3, OrbitPreview};
use rocketcon_sim::OrbitPreview as SessionPreview;

impl From<SessionPreview> for OrbitPreview {
    fn from(source: SessionPreview) -> Self {
        Self {
            reference_body_id: source.reference_body_id.to_string(),
            source_epoch_seconds: source.source_epoch_seconds,
            horizon_seconds: source.horizon_seconds,
            has_impact: source.impact_epoch_seconds.is_some(),
            impact_epoch_seconds: source.impact_epoch_seconds.unwrap_or_default(),
            under_thrust: source.under_thrust,
            has_eccentricity: source.eccentricity.is_some(),
            eccentricity: source.eccentricity.unwrap_or_default(),
            has_periapsis: source.periapsis_altitude_m.is_some(),
            periapsis_altitude_m: source.periapsis_altitude_m.unwrap_or_default(),
            has_apoapsis: source.apoapsis_altitude_m.is_some(),
            apoapsis_altitude_m: source.apoapsis_altitude_m.unwrap_or_default(),
            has_period: source.period_seconds.is_some(),
            period_seconds: source.period_seconds.unwrap_or_default(),
            has_next_periapsis: source.next_periapsis.is_some(),
            next_periapsis_seconds: source
                .next_periapsis
                .as_ref()
                .map_or(0.0, |event| event.time_to_seconds),
            next_periapsis_position_m: source
                .next_periapsis
                .as_ref()
                .map_or([0.0; 3], |event| event.relative_position_m)
                .into(),
            has_next_apoapsis: source.next_apoapsis.is_some(),
            next_apoapsis_seconds: source
                .next_apoapsis
                .as_ref()
                .map_or(0.0, |event| event.time_to_seconds),
            next_apoapsis_position_m: source
                .next_apoapsis
                .as_ref()
                .map_or([0.0; 3], |event| event.relative_position_m)
                .into(),
            relative_points_m: source
                .relative_points_m
                .into_iter()
                .map(FfiVec3::from)
                .collect(),
        }
    }
}
