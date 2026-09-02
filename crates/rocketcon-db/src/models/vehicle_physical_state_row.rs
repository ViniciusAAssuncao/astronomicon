use crate::error::RocketDbError;
use astronomicon_core::units::{
    AngularVelocityVector, Duration, Position, Quaternion, VelocityVector,
};
use rocketcon_core::domain::VehiclePhysicalState;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct VehiclePhysicalStateRow {
    pub vehicle_id: String,
    pub position_x_m: f64,
    pub position_y_m: f64,
    pub position_z_m: f64,
    pub velocity_x_m_s: f64,
    pub velocity_y_m_s: f64,
    pub velocity_z_m_s: f64,
    pub orientation_q_w: f64,
    pub orientation_q_x: f64,
    pub orientation_q_y: f64,
    pub orientation_q_z: f64,
    pub angular_velocity_x_rad_s: f64,
    pub angular_velocity_y_rad_s: f64,
    pub angular_velocity_z_rad_s: f64,
    pub reference_body_id: String,
    pub captured_universe_epoch_s: f64,
    pub captured_at_epoch_s: f64,
}

impl TryFrom<VehiclePhysicalStateRow> for VehiclePhysicalState {
    type Error = RocketDbError;

    fn try_from(row: VehiclePhysicalStateRow) -> Result<Self, Self::Error> {
        let vehicle_id = Uuid::parse_str(&row.vehicle_id)?;
        let reference_body_id = Uuid::parse_str(&row.reference_body_id)?;

        let position =
            Position::from_components(row.position_x_m, row.position_y_m, row.position_z_m);
        let velocity =
            VelocityVector::from_components(row.velocity_x_m_s, row.velocity_y_m_s, row.velocity_z_m_s);
        let orientation = Quaternion::new(
            row.orientation_q_w,
            row.orientation_q_x,
            row.orientation_q_y,
            row.orientation_q_z,
        );
        let angular_velocity = AngularVelocityVector::from_components(
            row.angular_velocity_x_rad_s,
            row.angular_velocity_y_rad_s,
            row.angular_velocity_z_rad_s,
        );

        let state = VehiclePhysicalState::new(
            vehicle_id,
            position,
            velocity,
            orientation,
            angular_velocity,
            reference_body_id,
            Duration::new(row.captured_universe_epoch_s),
            Duration::new(row.captured_at_epoch_s),
        )?;

        Ok(state)
    }
}