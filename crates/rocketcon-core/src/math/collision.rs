use astronomicon_core::math::reference_frames::{
    geodetic_altitude_and_normal, inertial_to_body_fixed_position,
};
use astronomicon_core::units::{
    AngularVelocityVector, Length, Position, Quaternion, Vector3, VelocityVector,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SurfaceContactState {
    pub has_contact: bool,
    pub geodetic_altitude: Length,
    pub surface_normal_world: Vector3,
    pub impact_relative_velocity: VelocityVector,
}

impl SurfaceContactState {
    pub fn new(
        has_contact: bool,
        geodetic_altitude: Length,
        surface_normal_world: Vector3,
        impact_relative_velocity: VelocityVector,
    ) -> Self {
        Self {
            has_contact,
            geodetic_altitude,
            surface_normal_world,
            impact_relative_velocity,
        }
    }

    pub fn has_contact(&self) -> bool {
        self.has_contact
    }

    pub fn geodetic_altitude(&self) -> Length {
        self.geodetic_altitude
    }

    pub fn surface_normal_world(&self) -> Vector3 {
        self.surface_normal_world
    }

    pub fn impact_relative_velocity(&self) -> VelocityVector {
        self.impact_relative_velocity
    }
}

pub fn resolve_surface_contact(
    equatorial_radius: Length,
    polar_radius: Length,
    planet_position: Position,
    planet_orientation: Quaternion,
    planet_angular_velocity_inertial: AngularVelocityVector,
    vehicle_position: Position,
    vehicle_velocity: VelocityVector,
) -> SurfaceContactState {
    let body_fixed_position = inertial_to_body_fixed_position(
        planet_position,
        planet_orientation,
        vehicle_position,
    );

    let (geodetic_altitude, normal_body) = geodetic_altitude_and_normal(
        equatorial_radius,
        polar_radius,
        body_fixed_position,
    );

    let surface_normal_world = planet_orientation.rotate_vector(normal_body).normalized();

    let r_rel = vehicle_position.raw() - planet_position.raw();
    let omega = planet_angular_velocity_inertial.raw();
    let v_corot = omega.cross(&r_rel);

    let v_rel = vehicle_velocity.raw() - v_corot;
    let impact_relative_velocity = VelocityVector::from_raw(v_rel);

    let has_contact = geodetic_altitude.value() <= 0.0;

    SurfaceContactState::new(
        has_contact,
        geodetic_altitude,
        surface_normal_world,
        impact_relative_velocity,
    )
}
