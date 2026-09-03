use crate::error::RocketDbError;
use astronomicon_core::units::{Angle, Duration, GravitationalParameter, Length};
use rocketcon_core::domain::TrajectoryPatch;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct TrajectoryPatchRow {
    pub id: String,
    pub vehicle_id: String,
    pub reference_body_id: String,
    pub start_universe_epoch_s: f64,
    pub end_universe_epoch_s: Option<f64>,
    pub semi_major_axis_m: f64,
    pub eccentricity: f64,
    pub inclination_rad: f64,
    pub longitude_of_ascending_node_rad: f64,
    pub argument_of_periapsis_rad: f64,
    pub true_anomaly_at_epoch_rad: f64,
    pub gravitational_parameter_m3_s2: f64,
}

impl TryFrom<TrajectoryPatchRow> for TrajectoryPatch {
    type Error = RocketDbError;

    fn try_from(row: TrajectoryPatchRow) -> Result<Self, Self::Error> {
        let id = Uuid::parse_str(&row.id)?;
        let vehicle_id = Uuid::parse_str(&row.vehicle_id)?;
        let reference_body_id = Uuid::parse_str(&row.reference_body_id)?;

        let patch = TrajectoryPatch::new(
            id,
            vehicle_id,
            reference_body_id,
            Duration::new(row.start_universe_epoch_s),
            row.end_universe_epoch_s.map(Duration::new),
            Length::new(row.semi_major_axis_m),
            row.eccentricity,
            Angle::new(row.inclination_rad),
            Angle::new(row.longitude_of_ascending_node_rad),
            Angle::new(row.argument_of_periapsis_rad),
            Angle::new(row.true_anomaly_at_epoch_rad),
            GravitationalParameter::new(row.gravitational_parameter_m3_s2),
        )?;

        Ok(patch)
    }
}