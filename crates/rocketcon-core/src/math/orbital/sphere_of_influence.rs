use astronomicon_core::units::{Density, Length, Mass, Position, VelocityVector};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub fn laplace_sphere_of_influence_radius(
    semi_major_axis: Length,
    body_mass: Mass,
    parent_mass: Mass,
) -> Length {
    let a = semi_major_axis.value();
    let m = body_mass.value();
    let big_m = parent_mass.value();

    if a <= 0.0
        || m <= 0.0
        || big_m <= 0.0
        || !a.is_finite()
        || !m.is_finite()
        || !big_m.is_finite()
    {
        return Length::new(0.0);
    }

    let ratio = m / big_m;
    let r_soi = a * ratio.powf(0.4);

    if !r_soi.is_finite() || r_soi <= 0.0 {
        Length::new(0.0)
    } else {
        Length::new(r_soi)
    }
}

pub fn hill_sphere_radius(
    semi_major_axis: Length,
    eccentricity: f64,
    body_mass: Mass,
    parent_mass: Mass,
) -> Length {
    astronomicon_core::math::stability::hill_sphere_radius(
        semi_major_axis,
        eccentricity,
        body_mass,
        parent_mass,
    )
}

pub fn is_position_within_soi(
    position: Position,
    body_position: Position,
    soi_radius: Length,
) -> bool {
    let r_soi = soi_radius.value();
    if r_soi <= 0.0 || !r_soi.is_finite() {
        return false;
    }
    let diff = position.raw() - body_position.raw();
    let dist_sq = diff.dot(&diff);
    dist_sq <= r_soi * r_soi
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CelestialBodySoi {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub position: Position,
    pub mass: Mass,
    pub soi_radius: Length,
    pub body_radius: Length,
    pub atmosphere_boundary_altitude: Option<Length>,
    pub atmosphere_scale_height: Option<Length>,
    pub atmosphere_surface_density: Option<Density>,
}

impl CelestialBodySoi {
    pub fn new(
        id: Uuid,
        parent_id: Option<Uuid>,
        position: Position,
        mass: Mass,
        soi_radius: Length,
    ) -> Self {
        Self {
            id,
            parent_id,
            position,
            mass,
            soi_radius,
            body_radius: Length::new(0.0),
            atmosphere_boundary_altitude: None,
            atmosphere_scale_height: None,
            atmosphere_surface_density: None,
        }
    }

    pub fn new_with_geometry(
        id: Uuid,
        parent_id: Option<Uuid>,
        position: Position,
        mass: Mass,
        soi_radius: Length,
        body_radius: Length,
    ) -> Self {
        Self {
            id,
            parent_id,
            position,
            mass,
            soi_radius,
            body_radius,
            atmosphere_boundary_altitude: None,
            atmosphere_scale_height: None,
            atmosphere_surface_density: None,
        }
    }

    pub fn with_atmosphere(
        mut self,
        body_radius: Length,
        atmosphere_boundary_altitude: Length,
        atmosphere_scale_height: Length,
        atmosphere_surface_density: Density,
    ) -> Self {
        self.body_radius = body_radius;
        self.atmosphere_boundary_altitude = Some(atmosphere_boundary_altitude);
        self.atmosphere_scale_height = Some(atmosphere_scale_height);
        self.atmosphere_surface_density = Some(atmosphere_surface_density);
        self
    }

    pub fn from_orbital_elements(
        id: Uuid,
        parent_id: Option<Uuid>,
        position: Position,
        body_mass: Mass,
        parent_mass: Mass,
        semi_major_axis: Length,
    ) -> Self {
        let soi_radius =
            laplace_sphere_of_influence_radius(semi_major_axis, body_mass, parent_mass);
        Self::new(id, parent_id, position, body_mass, soi_radius)
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn parent_id(&self) -> Option<Uuid> {
        self.parent_id
    }

    pub fn position(&self) -> Position {
        self.position
    }

    pub fn mass(&self) -> Mass {
        self.mass
    }

    pub fn soi_radius(&self) -> Length {
        self.soi_radius
    }

    pub fn body_radius(&self) -> Length {
        self.body_radius
    }

    pub fn atmosphere_boundary_altitude(&self) -> Option<Length> {
        self.atmosphere_boundary_altitude
    }

    pub fn atmosphere_scale_height(&self) -> Option<Length> {
        self.atmosphere_scale_height
    }

    pub fn atmosphere_surface_density(&self) -> Option<Density> {
        self.atmosphere_surface_density
    }

    pub fn atmospheric_entry_radius(&self) -> Option<Length> {
        self.atmosphere_boundary_altitude
            .map(|h| Length::new(self.body_radius.value() + h.value()))
    }

    pub fn has_atmosphere(&self) -> bool {
        self.atmosphere_boundary_altitude.is_some()
            && self.atmosphere_surface_density.is_some()
            && self.atmosphere_scale_height.is_some()
    }

    pub fn contains_position(&self, position: Position) -> bool {
        is_position_within_soi(position, self.position, self.soi_radius)
    }
}

pub fn resolve_active_soi_body(
    vehicle_position: Position,
    candidate_bodies: &[CelestialBodySoi],
    default_parent_id: Uuid,
) -> Uuid {
    let mut selected_id = default_parent_id;
    let mut min_radius = f64::INFINITY;

    for body in candidate_bodies {
        if body.contains_position(vehicle_position) {
            let r = body.soi_radius.value();
            if r < min_radius {
                min_radius = r;
                selected_id = body.id;
            }
        }
    }

    selected_id
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SoiTransitionEvent {
    pub previous_parent_id: Uuid,
    pub new_parent_id: Uuid,
}

impl SoiTransitionEvent {
    pub fn new(previous_parent_id: Uuid, new_parent_id: Uuid) -> Self {
        Self {
            previous_parent_id,
            new_parent_id,
        }
    }
}

pub fn check_soi_transition(
    current_parent_id: Uuid,
    vehicle_position: Position,
    candidate_bodies: &[CelestialBodySoi],
) -> Option<SoiTransitionEvent> {
    let resolved_id =
        resolve_active_soi_body(vehicle_position, candidate_bodies, current_parent_id);
    if resolved_id != current_parent_id {
        Some(SoiTransitionEvent::new(current_parent_id, resolved_id))
    } else {
        None
    }
}

pub fn transform_state_to_new_soi(
    vehicle_position: Position,
    vehicle_velocity: VelocityVector,
    new_body_position: Position,
    new_body_velocity: VelocityVector,
) -> (Position, VelocityVector) {
    let rel_pos = vehicle_position.raw() - new_body_position.raw();
    let rel_vel = vehicle_velocity.raw() - new_body_velocity.raw();
    (
        Position::from_raw(rel_pos),
        VelocityVector::from_raw(rel_vel),
    )
}

pub fn transform_state_from_old_soi(
    vehicle_relative_position: Position,
    vehicle_relative_velocity: VelocityVector,
    old_body_position: Position,
    old_body_velocity: VelocityVector,
) -> (Position, VelocityVector) {
    let abs_pos = vehicle_relative_position.raw() + old_body_position.raw();
    let abs_vel = vehicle_relative_velocity.raw() + old_body_velocity.raw();
    (
        Position::from_raw(abs_pos),
        VelocityVector::from_raw(abs_vel),
    )
}
