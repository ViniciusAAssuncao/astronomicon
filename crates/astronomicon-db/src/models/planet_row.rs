use crate::error::DbError;
use astronomicon_core::domain::{OrbitalElements, Planet, PlanetKind};
use astronomicon_core::error::DomainError;
use astronomicon_core::units::{Angle, Duration, Length, Mass};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct PlanetRow {
    pub id: String,
    pub parent_star_id: Option<String>,
    pub parent_planet_id: Option<String>,
    pub name: String,
    pub kind: String,
    pub mass_kg: f64,
    pub equatorial_radius_m: Option<f64>,
    pub polar_radius_m: Option<f64>,
    pub rotation_period_s: Option<f64>,
    pub axial_tilt_rad: Option<f64>,
    pub geometric_albedo: Option<f64>,
    pub bond_albedo: Option<f64>,
    pub thermal_inertia: Option<f64>,
    pub solstice_true_anomaly_rad: Option<f64>,
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
    mean_anomaly_at_epoch_rad: Option<f64>,
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
            field: "orbital_elements".to_string(),
            reason: "partial orbital elements provided".to_string(),
        })),
    }
}

impl TryFrom<PlanetRow> for Planet {
    type Error = DbError;

    fn try_from(row: PlanetRow) -> Result<Self, Self::Error> {
        let id = Uuid::parse_str(&row.id)?;
        let parent_star_id = row.parent_star_id.as_deref().map(Uuid::parse_str).transpose()?;
        let parent_planet_id = row.parent_planet_id.as_deref().map(Uuid::parse_str).transpose()?;

        let kind = match row.kind.as_str() {
            "Telluric" => PlanetKind::Telluric,
            "GasGiant" => PlanetKind::GasGiant,
            "IceGiant" => PlanetKind::IceGiant,
            "DwarfPlanet" => PlanetKind::DwarfPlanet,
            "Chthonian" => PlanetKind::Chthonian,
            "CarbonPlanet" => PlanetKind::CarbonPlanet,
            "IcyBody" => PlanetKind::IcyBody,
            "Exotic" => PlanetKind::Exotic,
            other => {
                return Err(DbError::Domain(DomainError::InvalidInvariant {
                    field: "kind".to_string(),
                    reason: format!("unknown planet kind: {}", other),
                }))
            }
        };

        let orbital_elements = parse_orbital_elements(
            row.semi_major_axis_m,
            row.eccentricity,
            row.inclination_rad,
            row.longitude_ascending_node_rad,
            row.argument_periapsis_rad,
            row.mean_anomaly_at_epoch_rad,
        )?;

        let planet = Planet::new(
            id,
            parent_star_id,
            parent_planet_id,
            row.name,
            kind,
            Mass::new(row.mass_kg),
            row.equatorial_radius_m.map(Length::new),
            row.polar_radius_m.map(Length::new),
            row.rotation_period_s.map(Duration::new),
            row.axial_tilt_rad.map(Angle::new),
            row.geometric_albedo,
            row.bond_albedo,
            row.thermal_inertia,
            row.solstice_true_anomaly_rad.map(Angle::new),
            orbital_elements,
        )?;

        Ok(planet)
    }
}