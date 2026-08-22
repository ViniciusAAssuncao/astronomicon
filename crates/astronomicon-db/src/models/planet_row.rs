use crate::error::DbError;
use astronomicon_core::domain::{ OrbitalElements, OrbitalParent, Planet, PlanetKind };
use astronomicon_core::error::DomainError;
use astronomicon_core::units::{ Angle, Duration, Length, MagneticFluxDensity, Mass };
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct PlanetRow {
    pub id: String,
    pub star_system_id: Option<String>,
    pub parent_star_id: Option<String>,
    pub parent_planet_id: Option<String>,
    pub parent_barycenter_id: Option<String>,
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
    pub oblateness_j2: Option<f64>,
    pub core_mass_fraction: Option<f64>,
    pub radioactive_heating_rate: Option<f64>,
    pub magnetic_field_locked: Option<f64>,
    pub love_number_k2: Option<f64>,
    pub tidal_dissipation_factor_q: Option<f64>,
    pub mantle_hydration_fraction: Option<f64>,
}

fn parse_orbital_parent(
    parent_star_id: Option<String>,
    parent_planet_id: Option<String>,
    parent_barycenter_id: Option<String>
) -> Result<OrbitalParent, DbError> {
    match (parent_star_id, parent_planet_id, parent_barycenter_id) {
        (None, None, None) => Ok(OrbitalParent::Fixed),
        (Some(id), None, None) => Ok(OrbitalParent::Star(Uuid::parse_str(&id)?)),
        (None, Some(id), None) => Ok(OrbitalParent::Planet(Uuid::parse_str(&id)?)),
        (None, None, Some(id)) => Ok(OrbitalParent::Barycenter(Uuid::parse_str(&id)?)),
        _ =>
            Err(
                DbError::Domain(DomainError::InvalidInvariant {
                    field: "orbital_parent".to_string(),
                    reason: "multiple orbital parents specified".to_string(),
                })
            ),
    }
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

impl TryFrom<PlanetRow> for Planet {
    type Error = DbError;

    fn try_from(row: PlanetRow) -> Result<Self, Self::Error> {
        let id = Uuid::parse_str(&row.id)?;
        let star_system_id = row.star_system_id.as_deref().map(Uuid::parse_str).transpose()?;

        let orbital_parent = parse_orbital_parent(
            row.parent_star_id,
            row.parent_planet_id,
            row.parent_barycenter_id
        )?;

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
                return Err(
                    DbError::Domain(DomainError::InvalidInvariant {
                        field: "kind".to_string(),
                        reason: format!("unknown planet kind: {}", other),
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

        let planet = Planet::builder(id, row.name, Mass::new(row.mass_kg), kind, orbital_parent)
            .with_star_system_id(star_system_id)
            .with_equatorial_radius(row.equatorial_radius_m.map(Length::new))
            .with_polar_radius(row.polar_radius_m.map(Length::new))
            .with_rotation_period(row.rotation_period_s.map(Duration::new))
            .with_obliquity(row.axial_tilt_rad.map(Angle::new))
            .with_geometric_albedo(row.geometric_albedo)
            .with_bond_albedo(row.bond_albedo)
            .with_thermal_inertia(row.thermal_inertia)
            .with_solstice_true_anomaly(row.solstice_true_anomaly_rad.map(Angle::new))
            .with_orbital_elements(orbital_elements)
            .with_oblateness_j2(row.oblateness_j2)
            .with_core_mass_fraction(row.core_mass_fraction)
            .with_radioactive_heating_rate(row.radioactive_heating_rate)
            .with_magnetic_field_locked(row.magnetic_field_locked.map(MagneticFluxDensity::new))
            .with_love_number_k2(row.love_number_k2)
            .with_tidal_dissipation_factor_q(row.tidal_dissipation_factor_q)
            .with_mantle_hydration_fraction(row.mantle_hydration_fraction)
            .build()?;

        Ok(planet)
    }
}
