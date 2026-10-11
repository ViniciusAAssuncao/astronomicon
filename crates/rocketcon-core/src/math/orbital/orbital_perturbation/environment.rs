use super::harmonics::zonal_harmonics_acceleration_inertial;
use super::srp::srp_acceleration;
use super::third_body::accumulated_third_body_perturbations;
use super::types::ZonalHarmonics;
use crate::math::mass_properties::VehicleOpticalSurfaceProperties;
use astronomicon_core::units::{
    AccelerationVector, Density, GravitationalParameter, Length, Luminosity, Mass, Position,
    Quaternion, Vector3, VelocityVector,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerturbedEnvironment {
    pub primary_position: Position,
    pub primary_orientation: Quaternion,
    pub primary_mu: GravitationalParameter,
    pub primary_equatorial_radius: Length,
    pub primary_j2: Option<f64>,
    pub primary_harmonics: ZonalHarmonics,
    pub third_bodies: Vec<(Vector3, Mass)>,
    pub star_position: Option<Position>,
    pub star_luminosity: Option<Luminosity>,
    pub atmosphere_scale_height: Option<Length>,
    pub atmosphere_surface_density: Option<Density>,
}

impl PerturbedEnvironment {
    pub fn new(
        primary_position: Position,
        primary_orientation: Quaternion,
        primary_mu: GravitationalParameter,
        primary_equatorial_radius: Length,
        primary_j2: Option<f64>,
        third_bodies: Vec<(Vector3, Mass)>,
    ) -> Self {
        let j2_val = primary_j2.unwrap_or(0.0);
        Self {
            primary_position,
            primary_orientation,
            primary_mu,
            primary_equatorial_radius,
            primary_j2,
            primary_harmonics: ZonalHarmonics::j2_only(j2_val),
            third_bodies,
            star_position: None,
            star_luminosity: None,
            atmosphere_scale_height: None,
            atmosphere_surface_density: None,
        }
    }

    pub fn with_harmonics(mut self, harmonics: ZonalHarmonics) -> Self {
        self.primary_harmonics = harmonics;
        self.primary_j2 = Some(harmonics.j2);
        self
    }

    pub fn with_star(mut self, star_position: Position, star_luminosity: Luminosity) -> Self {
        self.star_position = Some(star_position);
        self.star_luminosity = Some(star_luminosity);
        self
    }

    pub fn with_atmosphere(
        mut self,
        atmosphere_scale_height: Length,
        atmosphere_surface_density: Density,
    ) -> Self {
        self.atmosphere_scale_height = Some(atmosphere_scale_height);
        self.atmosphere_surface_density = Some(atmosphere_surface_density);
        self
    }

    pub fn gravitational_acceleration_at(&self, position: Position) -> AccelerationVector {
        let mu_val = self.primary_mu.value();
        if mu_val <= 0.0 || !mu_val.is_finite() {
            return AccelerationVector::zero();
        }

        let r_rel_inertial = position.raw() - self.primary_position.raw();
        let dist = r_rel_inertial.magnitude();
        if dist <= 1e-3 || !dist.is_finite() {
            return AccelerationVector::zero();
        }

        let a_pm_raw = -r_rel_inertial * (mu_val / (dist * dist * dist));
        let a_pm = AccelerationVector::from_raw(a_pm_raw);

        let a_zonal = zonal_harmonics_acceleration_inertial(
            self.primary_mu,
            self.primary_equatorial_radius,
            &self.primary_harmonics,
            self.primary_position,
            position,
            self.primary_orientation,
        );

        let a_third = accumulated_third_body_perturbations(r_rel_inertial, &self.third_bodies);

        a_pm + a_zonal + a_third
    }

    pub fn srp_acceleration_at(
        &self,
        position: Position,
        mass: Mass,
        effective_srp_area_m2: f64,
        cr: f64,
    ) -> AccelerationVector {
        match (self.star_position, self.star_luminosity) {
            (Some(star_pos), Some(star_lum)) => srp_acceleration(
                position,
                star_pos,
                star_lum,
                self.primary_position,
                self.primary_equatorial_radius,
                effective_srp_area_m2,
                cr,
                mass,
            ),
            _ => AccelerationVector::zero(),
        }
    }

    pub fn total_perturbation_acceleration_at(
        &self,
        position: Position,
        _velocity: VelocityVector,
        mass: Mass,
        optical_properties: &VehicleOpticalSurfaceProperties,
    ) -> AccelerationVector {
        let a_grav = self.gravitational_acceleration_at(position);
        let a_srp = self.srp_acceleration_at(
            position,
            mass,
            optical_properties.effective_srp_area_m2,
            optical_properties.radiation_pressure_coefficient,
        );
        a_grav + a_srp
    }
}

pub fn primary_gravitational_acceleration_with_j2(
    vehicle_position_inertial: Position,
    primary_position_inertial: Position,
    primary_orientation: Quaternion,
    primary_mu: GravitationalParameter,
    equatorial_radius: Length,
    j2: Option<f64>,
) -> AccelerationVector {
    let harmonics = ZonalHarmonics::j2_only(j2.unwrap_or(0.0));
    let env = PerturbedEnvironment::new(
        primary_position_inertial,
        primary_orientation,
        primary_mu,
        equatorial_radius,
        j2,
        Vec::new(),
    )
    .with_harmonics(harmonics);

    env.gravitational_acceleration_at(vehicle_position_inertial)
}
