use crate::error::DbError;
use crate::models::orbital_parsing::{parse_orbital_elements, parse_orbital_parent};
use astronomicon_core::domain::{Barycenter, BarycenterMember, OrbitalElements};
use astronomicon_core::error::DomainError;
use astronomicon_core::units::{Angle, Length};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct BarycenterRow {
    pub id: String,
    pub star_system_id: Option<String>,
    pub name: String,
    pub primary_star_id: Option<String>,
    pub primary_planet_id: Option<String>,
    pub primary_barycenter_id: Option<String>,
    pub secondary_star_id: Option<String>,
    pub secondary_planet_id: Option<String>,
    pub secondary_barycenter_id: Option<String>,
    pub internal_semi_major_axis_m: f64,
    pub internal_eccentricity: f64,
    pub internal_inclination_rad: f64,
    pub internal_longitude_ascending_node_rad: f64,
    pub internal_argument_periapsis_rad: f64,
    pub internal_mean_anomaly_at_epoch_rad: f64,
    pub parent_star_id: Option<String>,
    pub parent_planet_id: Option<String>,
    pub parent_barycenter_id: Option<String>,
    pub parent_minor_planet_id: Option<String>,
    pub external_semi_major_axis_m: Option<f64>,
    pub external_eccentricity: Option<f64>,
    pub external_inclination_rad: Option<f64>,
    pub external_longitude_ascending_node_rad: Option<f64>,
    pub external_argument_periapsis_rad: Option<f64>,
    pub external_mean_anomaly_at_epoch_rad: Option<f64>,
}

fn parse_member(
    star_id: Option<String>,
    planet_id: Option<String>,
    barycenter_id: Option<String>,
    slot_name: &str,
) -> Result<BarycenterMember, DbError> {
    match (star_id, planet_id, barycenter_id) {
        (Some(id), None, None) => Ok(BarycenterMember::Star(Uuid::parse_str(&id)?)),
        (None, Some(id), None) => Ok(BarycenterMember::Planet(Uuid::parse_str(&id)?)),
        (None, None, Some(id)) => Ok(BarycenterMember::Barycenter(Uuid::parse_str(&id)?)),
        _ => Err(DbError::Domain(DomainError::InvalidInvariant {
            field: slot_name.to_string(),
            reason: "exactly one member reference must be set".to_string(),
        })),
    }
}

impl TryFrom<BarycenterRow> for Barycenter {
    type Error = DbError;

    fn try_from(row: BarycenterRow) -> Result<Self, Self::Error> {
        let id = Uuid::parse_str(&row.id)?;
        let star_system_id = row
            .star_system_id
            .as_deref()
            .map(Uuid::parse_str)
            .transpose()?;

        let member_primary = parse_member(
            row.primary_star_id,
            row.primary_planet_id,
            row.primary_barycenter_id,
            "member_primary",
        )?;

        let member_secondary = parse_member(
            row.secondary_star_id,
            row.secondary_planet_id,
            row.secondary_barycenter_id,
            "member_secondary",
        )?;

        let internal_orbital_elements = OrbitalElements::new(
            Length::new(row.internal_semi_major_axis_m),
            row.internal_eccentricity,
            Angle::new(row.internal_inclination_rad),
            Angle::new(row.internal_longitude_ascending_node_rad),
            Angle::new(row.internal_argument_periapsis_rad),
            Angle::new(row.internal_mean_anomaly_at_epoch_rad),
        )?;

        let orbital_parent = parse_orbital_parent(
            row.parent_star_id,
            row.parent_planet_id,
            row.parent_barycenter_id,
            row.parent_minor_planet_id,
        )?;

        let external_orbital_elements = parse_orbital_elements(
            row.external_semi_major_axis_m,
            row.external_eccentricity,
            row.external_inclination_rad,
            row.external_longitude_ascending_node_rad,
            row.external_argument_periapsis_rad,
            row.external_mean_anomaly_at_epoch_rad,
            "external_orbital_elements",
            "partial external orbital elements provided",
        )?;

        let barycenter = Barycenter::new(
            id,
            star_system_id,
            row.name,
            member_primary,
            member_secondary,
            internal_orbital_elements,
            orbital_parent,
            external_orbital_elements,
        )?;

        Ok(barycenter)
    }
}
