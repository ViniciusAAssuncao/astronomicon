use crate::error::DbError;
use crate::models::orbital_parsing::{parse_orbital_elements, parse_orbital_parent};
use astronomicon_core::domain::{Star, StarKind};
use astronomicon_core::error::DomainError;
use astronomicon_core::units::{Angle, Duration, Length, Mass, Temperature};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct StarRow {
    pub id: String,
    pub star_system_id: Option<String>,
    pub parent_star_id: Option<String>,
    pub parent_planet_id: Option<String>,
    pub parent_barycenter_id: Option<String>,
    pub parent_minor_planet_id: Option<String>,
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
    pub oblateness_j2: Option<f64>,
    pub metallicity: Option<f64>,
}

impl TryFrom<StarRow> for Star {
    type Error = DbError;

    fn try_from(row: StarRow) -> Result<Self, Self::Error> {
        let id = Uuid::parse_str(&row.id)?;
        let star_system_id = row
            .star_system_id
            .as_deref()
            .map(Uuid::parse_str)
            .transpose()?;

        let orbital_parent = parse_orbital_parent(
            row.parent_star_id,
            row.parent_planet_id,
            row.parent_barycenter_id,
            row.parent_minor_planet_id,
        )?;

        let kind = match row.kind.as_str() {
            "Star" => StarKind::Star,
            "WhiteDwarf" => StarKind::WhiteDwarf,
            "NeutronStar" => StarKind::NeutronStar,
            "BlackHole" => StarKind::BlackHole,
            "BrownDwarf" => StarKind::BrownDwarf,
            "Exotic" => StarKind::Exotic,
            other => {
                return Err(DbError::Domain(DomainError::InvalidInvariant {
                    field: "kind".to_string(),
                    reason: format!("unknown star kind: {}", other),
                }));
            }
        };

        let orbital_elements = parse_orbital_elements(
            row.semi_major_axis_m,
            row.eccentricity,
            row.inclination_rad,
            row.longitude_ascending_node_rad,
            row.argument_periapsis_rad,
            row.mean_anomaly_at_epoch_rad,
            "orbital_elements",
            "partial orbital elements provided",
        )?;

        let star = Star::builder(id, row.name, Mass::new(row.mass_kg), kind, orbital_parent)
            .with_star_system_id(star_system_id)
            .with_radius(row.radius_m.map(Length::new))
            .with_effective_temperature(row.effective_temperature_k.map(Temperature::new))
            .with_rotation_period(row.rotation_period_s.map(Duration::new))
            .with_obliquity(row.axial_tilt_rad.map(Angle::new))
            .with_orbital_elements(orbital_elements)
            .with_oblateness_j2(row.oblateness_j2)
            .with_metallicity(row.metallicity)
            .build()?;

        Ok(star)
    }
}
