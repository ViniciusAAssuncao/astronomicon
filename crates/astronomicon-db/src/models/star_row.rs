use crate::error::DbError;
use astronomicon_core::domain::{ OrbitalElements, Star, StarKind };
use astronomicon_core::error::DomainError;
use astronomicon_core::units::{ Angle, Duration, Length, Mass, Temperature };
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct StarRow {
    pub id: String,
    pub star_system_id: Option<String>,
    pub name: String,
    pub kind: String,
    pub mass_kg: f64,
    pub radius_m: Option<f64>,
    pub effective_temperature_k: Option<f64>,
    pub rotation_period_s: Option<f64>,
    pub axial_tilt_rad: Option<f64>,
    pub semi_major_axis_m: Option<f64>,
    pub eccentricity: Option<f64>,
    pub inclination_rad: Option<f64>,
    pub longitude_ascending_node_rad: Option<f64>,
    pub argument_periapsis_rad: Option<f64>,
    pub mean_anomaly_at_epoch_rad: Option<f64>,
}

fn parse_orbital_elements(
    semi_major_axis_m: Option<f64>,
    eccentricity: Option<f64>,
    inclination_rad: Option<f64>,
    longitude_ascending_node_rad: Option<f64>,
    argument_periapsis_rad: Option<f64>,
    mean_anomaly_at_epoch_rad: Option<f64>
) -> Result<Option<OrbitalElements>, DbError> {
    match
        (
            semi_major_axis_m,
            eccentricity,
            inclination_rad,
            longitude_ascending_node_rad,
            argument_periapsis_rad,
            mean_anomaly_at_epoch_rad,
        )
    {
        (None, None, None, None, None, None) => Ok(None),
        (Some(a), Some(e), Some(inc), Some(lan), Some(arg), Some(m0)) => {
            let elements = OrbitalElements::new(
                Length::new(a),
                e,
                Angle::new(inc),
                Angle::new(lan),
                Angle::new(arg),
                Angle::new(m0)
            )?;
            Ok(Some(elements))
        }
        _ =>
            Err(
                DbError::Domain(DomainError::InvalidInvariant {
                    field: "orbital_elements".to_string(),
                    reason: "partial orbital elements provided".to_string(),
                })
            ),
    }
}

impl TryFrom<StarRow> for Star {
    type Error = DbError;

    fn try_from(row: StarRow) -> Result<Self, Self::Error> {
        let id = Uuid::parse_str(&row.id)?;
        let star_system_id = row.star_system_id.as_deref().map(Uuid::parse_str).transpose()?;

        let kind = match row.kind.as_str() {
            "Star" => StarKind::Star,
            "WhiteDwarf" => StarKind::WhiteDwarf,
            "NeutronStar" => StarKind::NeutronStar,
            "BlackHole" => StarKind::BlackHole,
            "BrownDwarf" => StarKind::BrownDwarf,
            "Exotic" => StarKind::Exotic,
            other => {
                return Err(
                    DbError::Domain(DomainError::InvalidInvariant {
                        field: "kind".to_string(),
                        reason: format!("unknown star kind: {}", other),
                    })
                );
            }
        };

        let orbital_elements = parse_orbital_elements(
            row.semi_major_axis_m,
            row.eccentricity,
            row.inclination_rad,
            row.longitude_ascending_node_rad,
            row.argument_periapsis_rad,
            row.mean_anomaly_at_epoch_rad
        )?;

        let star = Star::new(
            id,
            star_system_id,
            kind,
            row.name,
            Mass::new(row.mass_kg),
            row.radius_m.map(Length::new),
            row.effective_temperature_k.map(Temperature::new),
            row.rotation_period_s.map(Duration::new),
            row.axial_tilt_rad.map(Angle::new),
            orbital_elements
        )?;

        Ok(star)
    }
}
