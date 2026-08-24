use crate::error::DbError;
use crate::models::orbital_parsing::{parse_orbital_elements, parse_orbital_parent};
use astronomicon_core::domain::{MinorPlanet, SpectralType};
use astronomicon_core::error::DomainError;
use astronomicon_core::units::{Angle, Duration, Length, Mass};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct MinorPlanetRow {
    pub id: String,
    pub star_system_id: Option<String>,
    pub parent_star_id: Option<String>,
    pub parent_planet_id: Option<String>,
    pub parent_barycenter_id: Option<String>,
    pub parent_minor_planet_id: Option<String>,
    pub name: String,
    pub spectral_type: String,
    pub mass_kg: f64,
    pub axis_a_m: Option<f64>,
    pub axis_b_m: Option<f64>,
    pub axis_c_m: Option<f64>,
    pub rotation_period_s: Option<f64>,
    pub axial_tilt_rad: Option<f64>,
    pub macroporosity: Option<f64>,
    pub geometric_albedo: Option<f64>,
    pub bond_albedo: Option<f64>,
    pub semi_major_axis_m: Option<f64>,
    pub eccentricity: Option<f64>,
    pub inclination_rad: Option<f64>,
    pub longitude_ascending_node_rad: Option<f64>,
    pub argument_periapsis_rad: Option<f64>,
    pub mean_anomaly_at_epoch_rad: Option<f64>,
}

impl TryFrom<MinorPlanetRow> for MinorPlanet {
    type Error = DbError;

    fn try_from(row: MinorPlanetRow) -> Result<Self, Self::Error> {
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

        let spectral_type = match row.spectral_type.as_str() {
            "C" => SpectralType::C,
            "S" => SpectralType::S,
            "M" => SpectralType::M,
            "D" => SpectralType::D,
            "V" => SpectralType::V,
            "P" => SpectralType::P,
            other => {
                return Err(DbError::Domain(DomainError::InvalidInvariant {
                    field: "spectral_type".to_string(),
                    reason: format!("unknown spectral type: {}", other),
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

        let minor_planet = MinorPlanet::builder(
            id,
            row.name,
            spectral_type,
            Mass::new(row.mass_kg),
            orbital_parent,
        )
        .with_star_system_id(star_system_id)
        .with_axis_a(row.axis_a_m.map(Length::new))
        .with_axis_b(row.axis_b_m.map(Length::new))
        .with_axis_c(row.axis_c_m.map(Length::new))
        .with_rotation_period(row.rotation_period_s.map(Duration::new))
        .with_obliquity(row.axial_tilt_rad.map(Angle::new))
        .with_macroporosity(row.macroporosity)
        .with_geometric_albedo(row.geometric_albedo)
        .with_bond_albedo(row.bond_albedo)
        .with_orbital_elements(orbital_elements)
        .build()?;

        Ok(minor_planet)
    }
}
