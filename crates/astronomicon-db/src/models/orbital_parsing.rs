use crate::error::DbError;
use astronomicon_core::domain::{OrbitalElements, OrbitalParent};
use astronomicon_core::error::DomainError;
use astronomicon_core::units::{Angle, Length};
use uuid::Uuid;

pub fn parse_orbital_parent(
    parent_star_id: Option<String>,
    parent_planet_id: Option<String>,
    parent_barycenter_id: Option<String>,
    parent_minor_planet_id: Option<String>,
) -> Result<OrbitalParent, DbError> {
    match (
        parent_star_id,
        parent_planet_id,
        parent_barycenter_id,
        parent_minor_planet_id,
    ) {
        (None, None, None, None) => Ok(OrbitalParent::Fixed),
        (Some(id), None, None, None) => Ok(OrbitalParent::Star(Uuid::parse_str(&id)?)),
        (None, Some(id), None, None) => Ok(OrbitalParent::Planet(Uuid::parse_str(&id)?)),
        (None, None, Some(id), None) => Ok(OrbitalParent::Barycenter(Uuid::parse_str(&id)?)),
        (None, None, None, Some(id)) => Ok(OrbitalParent::MinorPlanet(Uuid::parse_str(&id)?)),
        _ => Err(DbError::Domain(DomainError::InvalidInvariant {
            field: "orbital_parent".to_string(),
            reason: "multiple orbital parents specified".to_string(),
        })),
    }
}

pub fn parse_orbital_elements(
    semi_major_axis_m: Option<f64>,
    eccentricity: Option<f64>,
    inclination_rad: Option<f64>,
    longitude_ascending_node_rad: Option<f64>,
    argument_periapsis_rad: Option<f64>,
    mean_anomaly_at_epoch_rad: Option<f64>,
    err_field: &str,
    err_reason: &str,
) -> Result<Option<OrbitalElements>, DbError> {
    match (
        semi_major_axis_m,
        eccentricity,
        inclination_rad,
        longitude_ascending_node_rad,
        argument_periapsis_rad,
        mean_anomaly_at_epoch_rad,
    ) {
        (None, None, None, None, None, None) => Ok(None),
        (Some(a), Some(e), Some(inc), Some(lan), Some(arg), Some(m0)) => {
            let elements = OrbitalElements::new(
                Length::new(a),
                e,
                Angle::new(inc),
                Angle::new(lan),
                Angle::new(arg),
                Angle::new(m0),
            )?;
            Ok(Some(elements))
        }
        _ => Err(DbError::Domain(DomainError::InvalidInvariant {
            field: err_field.to_string(),
            reason: err_reason.to_string(),
        })),
    }
}
