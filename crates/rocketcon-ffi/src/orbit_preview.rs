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
            relative_points_m: source
                .relative_points_m
                .into_iter()
                .map(FfiVec3::from)
                .collect(),
        }
    }
}
