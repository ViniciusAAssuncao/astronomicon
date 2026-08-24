use crate::domain::orbital_elements::OrbitalElements;
use crate::domain::orbital_parent::OrbitalParent;
use crate::domain::rheology::PlanetRheology;
use crate::domain::validation::{
    validate_finite, validate_non_negative_finite, validate_not_empty, validate_positive_finite,
    validate_unit_interval,
};
use crate::error::{DomainError, DomainResult};
use crate::units::{Angle, Duration, Length, MagneticFluxDensity, Mass};
use serde::{Deserialize, Serialize};
use std::f64::consts::TAU;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TectonicRegime {
    StagnantLid,
    PlateTectonics,
    HeatPipe,
    IceTectonics,
    Inactive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanetKind {
    Telluric,
    GasGiant,
    IceGiant,
    DwarfPlanet,
    Chthonian,
    CarbonPlanet,
    IcyBody,
    Exotic,
}

#[derive(Debug, Clone)]
pub struct PlanetBuilder {
    id: Uuid,
    name: String,
    mass: Mass,
    kind: PlanetKind,
    orbital_parent: OrbitalParent,
    star_system_id: Option<Uuid>,
    equatorial_radius: Option<Length>,
    polar_radius: Option<Length>,
    rotation_period: Option<Duration>,
    obliquity: Option<Angle>,
    geometric_albedo: Option<f64>,
    bond_albedo: Option<f64>,
    thermal_inertia: Option<f64>,
    solstice_true_anomaly: Option<Angle>,
    orbital_elements: Option<OrbitalElements>,
    oblateness_j2: Option<f64>,
    core_mass_fraction: Option<f64>,
    radioactive_heating_rate: Option<f64>,
    magnetic_field_locked: Option<MagneticFluxDensity>,
    love_number_k2: Option<f64>,
    tidal_dissipation_factor_q: Option<f64>,
    mantle_hydration_fraction: Option<f64>,
    dust_availability_factor: Option<f64>,
    surface_roughness: Option<Length>,
    rheology: Option<PlanetRheology>,
}

impl PlanetBuilder {
    pub fn new(
        id: Uuid,
        name: impl Into<String>,
        mass: Mass,
        kind: PlanetKind,
        orbital_parent: OrbitalParent,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            mass,
            kind,
            orbital_parent,
            star_system_id: None,
            equatorial_radius: None,
            polar_radius: None,
            rotation_period: None,
            obliquity: None,
            geometric_albedo: None,
            bond_albedo: None,
            thermal_inertia: None,
            solstice_true_anomaly: None,
            orbital_elements: None,
            oblateness_j2: None,
            core_mass_fraction: None,
            radioactive_heating_rate: None,
            magnetic_field_locked: None,
            love_number_k2: None,
            tidal_dissipation_factor_q: None,
            mantle_hydration_fraction: None,
            dust_availability_factor: None,
            surface_roughness: None,
            rheology: None,
        }
    }

    pub fn with_star_system_id(mut self, star_system_id: impl Into<Option<Uuid>>) -> Self {
        self.star_system_id = star_system_id.into();
        self
    }

    pub fn with_equatorial_radius(mut self, equatorial_radius: impl Into<Option<Length>>) -> Self {
        self.equatorial_radius = equatorial_radius.into();
        self
    }

    pub fn with_polar_radius(mut self, polar_radius: impl Into<Option<Length>>) -> Self {
        self.polar_radius = polar_radius.into();
        self
    }

    pub fn with_rotation_period(mut self, rotation_period: impl Into<Option<Duration>>) -> Self {
        self.rotation_period = rotation_period.into();
        self
    }

    pub fn with_obliquity(mut self, obliquity: impl Into<Option<Angle>>) -> Self {
        self.obliquity = obliquity.into();
        self
    }

    pub fn with_geometric_albedo(mut self, geometric_albedo: impl Into<Option<f64>>) -> Self {
        self.geometric_albedo = geometric_albedo.into();
        self
    }

    pub fn with_bond_albedo(mut self, bond_albedo: impl Into<Option<f64>>) -> Self {
        self.bond_albedo = bond_albedo.into();
        self
    }

    pub fn with_thermal_inertia(mut self, thermal_inertia: impl Into<Option<f64>>) -> Self {
        self.thermal_inertia = thermal_inertia.into();
        self
    }

    pub fn with_solstice_true_anomaly(
        mut self,
        solstice_true_anomaly: impl Into<Option<Angle>>,
    ) -> Self {
        self.solstice_true_anomaly = solstice_true_anomaly.into();
        self
    }

    pub fn with_orbital_elements(
        mut self,
        orbital_elements: impl Into<Option<OrbitalElements>>,
    ) -> Self {
        self.orbital_elements = orbital_elements.into();
        self
    }

    pub fn with_oblateness_j2(mut self, oblateness_j2: impl Into<Option<f64>>) -> Self {
        self.oblateness_j2 = oblateness_j2.into();
        self
    }

    pub fn with_core_mass_fraction(mut self, core_mass_fraction: impl Into<Option<f64>>) -> Self {
        self.core_mass_fraction = core_mass_fraction.into();
        self
    }

    pub fn with_radioactive_heating_rate(
        mut self,
        radioactive_heating_rate: impl Into<Option<f64>>,
    ) -> Self {
        self.radioactive_heating_rate = radioactive_heating_rate.into();
        self
    }

    pub fn with_magnetic_field_locked(
        mut self,
        magnetic_field_locked: impl Into<Option<MagneticFluxDensity>>,
    ) -> Self {
        self.magnetic_field_locked = magnetic_field_locked.into();
        self
    }

    pub fn with_love_number_k2(mut self, love_number_k2: impl Into<Option<f64>>) -> Self {
        self.love_number_k2 = love_number_k2.into();
        self
    }

    pub fn with_tidal_dissipation_factor_q(
        mut self,
        tidal_dissipation_factor_q: impl Into<Option<f64>>,
    ) -> Self {
        self.tidal_dissipation_factor_q = tidal_dissipation_factor_q.into();
        self
    }

    pub fn with_mantle_hydration_fraction(
        mut self,
        mantle_hydration_fraction: impl Into<Option<f64>>,
    ) -> Self {
        self.mantle_hydration_fraction = mantle_hydration_fraction.into();
        self
    }

    pub fn with_dust_availability_factor(
        mut self,
        dust_availability_factor: impl Into<Option<f64>>,
    ) -> Self {
        self.dust_availability_factor = dust_availability_factor.into();
        self
    }

    pub fn with_surface_roughness(
        mut self,
        surface_roughness: impl Into<Option<Length>>,
    ) -> Self {
        self.surface_roughness = surface_roughness.into();
        self
    }

    pub fn with_rheology(mut self, rheology: impl Into<Option<PlanetRheology>>) -> Self {
        self.rheology = rheology.into();
        self
    }

    pub fn build(self) -> DomainResult<Planet> {
        validate_not_empty(&self.name, "name")?;
        validate_positive_finite(self.mass.value(), "mass")?;

        if self.orbital_parent == OrbitalParent::Fixed && self.orbital_elements.is_some() {
            return Err(DomainError::InvalidInvariant {
                field: "orbital_elements".to_string(),
                reason: "fixed planet cannot have orbital elements".to_string(),
            });
        }

        if self.orbital_parent != OrbitalParent::Fixed && self.orbital_elements.is_none() {
            return Err(DomainError::InvalidInvariant {
                field: "orbital_elements".to_string(),
                reason: "non-fixed orbiting planet must have orbital elements".to_string(),
            });
        }

        if let Some(r) = self.equatorial_radius {
            validate_positive_finite(r.value(), "equatorial_radius")?;
        }

        if let Some(r) = self.polar_radius {
            validate_positive_finite(r.value(), "polar_radius")?;
        }

        if let Some(rot) = self.rotation_period {
            validate_positive_finite(rot.value(), "rotation_period")?;
        }

        if let Some(ob) = self.obliquity {
            validate_finite(ob.value(), "obliquity")?;
        }

        if let Some(geo) = self.geometric_albedo {
            validate_unit_interval(geo, "geometric_albedo")?;
        }

        if let Some(bond) = self.bond_albedo {
            validate_unit_interval(bond, "bond_albedo")?;
        }

        if let Some(ti) = self.thermal_inertia {
            validate_unit_interval(ti, "thermal_inertia")?;
        }

        if let Some(sta) = self.solstice_true_anomaly {
            validate_finite(sta.value(), "solstice_true_anomaly")?;
        }

        if let Some(j2) = self.oblateness_j2 {
            validate_finite(j2, "oblateness_j2")?;
        }

        if let Some(cmf) = self.core_mass_fraction {
            validate_unit_interval(cmf, "core_mass_fraction")?;
        }

        if let Some(rhr) = self.radioactive_heating_rate {
            validate_non_negative_finite(rhr, "radioactive_heating_rate")?;
        }

        if let Some(b) = self.magnetic_field_locked {
            validate_non_negative_finite(b.value(), "magnetic_field_locked")?;
        }

        if let Some(k2) = self.love_number_k2 {
            validate_positive_finite(k2, "love_number_k2")?;
        }

        if let Some(q) = self.tidal_dissipation_factor_q {
            validate_positive_finite(q, "tidal_dissipation_factor_q")?;
        }

        if let Some(hf) = self.mantle_hydration_fraction {
            validate_unit_interval(hf, "mantle_hydration_fraction")?;
        }

        if let Some(daf) = self.dust_availability_factor {
            validate_unit_interval(daf, "dust_availability_factor")?;
        }

        if let Some(sr) = self.surface_roughness {
            validate_positive_finite(sr.value(), "surface_roughness")?;
        }

        let solstice_true_anomaly = self
            .solstice_true_anomaly
            .map(|angle| Angle::new(angle.value().rem_euclid(TAU)));

        Ok(Planet {
            id: self.id,
            star_system_id: self.star_system_id,
            orbital_parent: self.orbital_parent,
            kind: self.kind,
            name: self.name,
            mass: self.mass,
            equatorial_radius: self.equatorial_radius,
            polar_radius: self.polar_radius,
            rotation_period: self.rotation_period,
            obliquity: self.obliquity,
            geometric_albedo: self.geometric_albedo,
            bond_albedo: self.bond_albedo,
            thermal_inertia: self.thermal_inertia,
            solstice_true_anomaly,
            orbital_elements: self.orbital_elements,
            oblateness_j2: self.oblateness_j2,
            core_mass_fraction: self.core_mass_fraction,
            radioactive_heating_rate: self.radioactive_heating_rate,
            magnetic_field_locked: self.magnetic_field_locked,
            love_number_k2: self.love_number_k2,
            tidal_dissipation_factor_q: self.tidal_dissipation_factor_q,
            mantle_hydration_fraction: self.mantle_hydration_fraction,
            dust_availability_factor: self.dust_availability_factor,
            surface_roughness: self.surface_roughness,
            rheology: self.rheology,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Planet {
    id: Uuid,
    star_system_id: Option<Uuid>,
    orbital_parent: OrbitalParent,
    name: String,
    kind: PlanetKind,
    mass: Mass,
    equatorial_radius: Option<Length>,
    polar_radius: Option<Length>,
    rotation_period: Option<Duration>,
    obliquity: Option<Angle>,
    geometric_albedo: Option<f64>,
    bond_albedo: Option<f64>,
    thermal_inertia: Option<f64>,
    solstice_true_anomaly: Option<Angle>,
    orbital_elements: Option<OrbitalElements>,
    oblateness_j2: Option<f64>,
    core_mass_fraction: Option<f64>,
    radioactive_heating_rate: Option<f64>,
    magnetic_field_locked: Option<MagneticFluxDensity>,
    love_number_k2: Option<f64>,
    tidal_dissipation_factor_q: Option<f64>,
    mantle_hydration_fraction: Option<f64>,
    dust_availability_factor: Option<f64>,
    surface_roughness: Option<Length>,
    rheology: Option<PlanetRheology>,
}

impl Planet {
    pub fn builder(
        id: Uuid,
        name: impl Into<String>,
        mass: Mass,
        kind: PlanetKind,
        orbital_parent: OrbitalParent,
    ) -> PlanetBuilder {
        PlanetBuilder::new(id, name, mass, kind, orbital_parent)
    }

    pub fn new(
        id: Uuid,
        star_system_id: Option<Uuid>,
        orbital_parent: OrbitalParent,
        name: String,
        kind: PlanetKind,
        mass: Mass,
        equatorial_radius: Option<Length>,
        polar_radius: Option<Length>,
        rotation_period: Option<Duration>,
        obliquity: Option<Angle>,
        geometric_albedo: Option<f64>,
        bond_albedo: Option<f64>,
        thermal_inertia: Option<f64>,
        solstice_true_anomaly: Option<Angle>,
        orbital_elements: Option<OrbitalElements>,
        oblateness_j2: Option<f64>,
        core_mass_fraction: Option<f64>,
        radioactive_heating_rate: Option<f64>,
        magnetic_field_locked: Option<MagneticFluxDensity>,
        love_number_k2: Option<f64>,
        tidal_dissipation_factor_q: Option<f64>,
        mantle_hydration_fraction: Option<f64>,
        dust_availability_factor: Option<f64>,
        surface_roughness: Option<Length>,
        rheology: Option<PlanetRheology>,
    ) -> DomainResult<Self> {
        Self::builder(id, name, mass, kind, orbital_parent)
            .with_star_system_id(star_system_id)
            .with_equatorial_radius(equatorial_radius)
            .with_polar_radius(polar_radius)
            .with_rotation_period(rotation_period)
            .with_obliquity(obliquity)
            .with_geometric_albedo(geometric_albedo)
            .with_bond_albedo(bond_albedo)
            .with_thermal_inertia(thermal_inertia)
            .with_solstice_true_anomaly(solstice_true_anomaly)
            .with_orbital_elements(orbital_elements)
            .with_oblateness_j2(oblateness_j2)
            .with_core_mass_fraction(core_mass_fraction)
            .with_radioactive_heating_rate(radioactive_heating_rate)
            .with_magnetic_field_locked(magnetic_field_locked)
            .with_love_number_k2(love_number_k2)
            .with_tidal_dissipation_factor_q(tidal_dissipation_factor_q)
            .with_mantle_hydration_fraction(mantle_hydration_fraction)
            .with_dust_availability_factor(dust_availability_factor)
            .with_surface_roughness(surface_roughness)
            .with_rheology(rheology)
            .build()
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn star_system_id(&self) -> Option<Uuid> {
        self.star_system_id
    }

    pub fn orbital_parent(&self) -> OrbitalParent {
        self.orbital_parent
    }

    pub fn kind(&self) -> PlanetKind {
        self.kind
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn mass(&self) -> Mass {
        self.mass
    }

    pub fn equatorial_radius(&self) -> Option<Length> {
        self.equatorial_radius
    }

    pub fn polar_radius(&self) -> Option<Length> {
        self.polar_radius
    }

    pub fn rotation_period(&self) -> Option<Duration> {
        self.rotation_period
    }

    pub fn obliquity(&self) -> Option<Angle> {
        self.obliquity
    }

    pub fn geometric_albedo(&self) -> Option<f64> {
        self.geometric_albedo
    }

    pub fn bond_albedo(&self) -> Option<f64> {
        self.bond_albedo
    }

    pub fn thermal_inertia(&self) -> Option<f64> {
        self.thermal_inertia
    }

    pub fn solstice_true_anomaly(&self) -> Option<Angle> {
        self.solstice_true_anomaly
    }

    pub fn orbital_elements(&self) -> Option<OrbitalElements> {
        self.orbital_elements
    }

    pub fn oblateness_j2(&self) -> Option<f64> {
        self.oblateness_j2
    }

    pub fn core_mass_fraction(&self) -> Option<f64> {
        self.core_mass_fraction
    }

    pub fn radioactive_heating_rate(&self) -> Option<f64> {
        self.radioactive_heating_rate
    }

    pub fn magnetic_field_locked(&self) -> Option<MagneticFluxDensity> {
        self.magnetic_field_locked
    }

    pub fn love_number_k2(&self) -> Option<f64> {
        self.love_number_k2
    }

    pub fn tidal_dissipation_factor_q(&self) -> Option<f64> {
        self.tidal_dissipation_factor_q
    }

    pub fn mantle_hydration_fraction(&self) -> Option<f64> {
        self.mantle_hydration_fraction
    }

    pub fn dust_availability_factor(&self) -> Option<f64> {
        self.dust_availability_factor
    }

    pub fn surface_roughness(&self) -> Option<Length> {
        self.surface_roughness
    }

    pub fn rheology(&self) -> Option<&PlanetRheology> {
        self.rheology.as_ref()
    }
}