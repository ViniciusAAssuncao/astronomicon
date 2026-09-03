use crate::math::orbital::types::OsculatingElements;
use crate::math::propulsion_dynamics::VehiclePropulsionForces;
use crate::math::rigid_body_state::{RigidBodyDerivative, RigidBodyState};
use astronomicon_core::math::gravity::oblateness::j2_gravitational_acceleration;
use astronomicon_core::units::constants::GRAVITATIONAL_CONSTANT;
use astronomicon_core::units::{
    AccelerationVector, AngularAccelerationVector, AngularVelocity, AngularVelocityVector,
    ForceVector, GravitationalParameter, InertiaTensor, Length, Mass, Position, Quaternion, Speed,
    TorqueVector, Vector3,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GaussVariationalRates {
    pub semi_major_axis_rate: Speed,
    pub eccentricity_rate: f64,
    pub inclination_rate: AngularVelocity,
    pub raan_rate: AngularVelocity,
    pub argument_of_periapsis_rate: AngularVelocity,
    pub true_anomaly_rate: AngularVelocity,
}

impl GaussVariationalRates {
    pub fn new(
        semi_major_axis_rate: Speed,
        eccentricity_rate: f64,
        inclination_rate: AngularVelocity,
        raan_rate: AngularVelocity,
        argument_of_periapsis_rate: AngularVelocity,
        true_anomaly_rate: AngularVelocity,
    ) -> Self {
        Self {
            semi_major_axis_rate,
            eccentricity_rate,
            inclination_rate,
            raan_rate,
            argument_of_periapsis_rate,
            true_anomaly_rate,
        }
    }

    pub fn zero() -> Self {
        Self {
            semi_major_axis_rate: Speed::new(0.0),
            eccentricity_rate: 0.0,
            inclination_rate: AngularVelocity::new(0.0),
            raan_rate: AngularVelocity::new(0.0),
            argument_of_periapsis_rate: AngularVelocity::new(0.0),
            true_anomaly_rate: AngularVelocity::new(0.0),
        }
    }
}

pub fn inertial_to_rsw_acceleration(
    position: Vector3,
    velocity: Vector3,
    acceleration_inertial: Vector3,
) -> Vector3 {
    let r_mag = position.magnitude();
    if r_mag < 1e-12 {
        return Vector3::zero();
    }

    let u_r = position / r_mag;
    let h_vec = position.cross(&velocity);
    let h_mag = h_vec.magnitude();

    if h_mag < 1e-12 {
        let u_perp = u_r.any_perpendicular();
        let u_norm = u_r.cross(&u_perp).normalized();
        return Vector3::new(
            acceleration_inertial.dot(&u_r),
            acceleration_inertial.dot(&u_perp),
            acceleration_inertial.dot(&u_norm),
        );
    }

    let u_h = h_vec / h_mag;
    let u_theta = u_h.cross(&u_r).normalized();

    Vector3::new(
        acceleration_inertial.dot(&u_r),
        acceleration_inertial.dot(&u_theta),
        acceleration_inertial.dot(&u_h),
    )
}

pub fn rsw_to_inertial_acceleration(
    position: Vector3,
    velocity: Vector3,
    acceleration_rsw: Vector3,
) -> Vector3 {
    let r_mag = position.magnitude();
    if r_mag < 1e-12 {
        return Vector3::zero();
    }

    let u_r = position / r_mag;
    let h_vec = position.cross(&velocity);
    let h_mag = h_vec.magnitude();

    if h_mag < 1e-12 {
        let u_perp = u_r.any_perpendicular();
        let u_norm = u_r.cross(&u_perp).normalized();
        return u_r * acceleration_rsw.0
            + u_perp * acceleration_rsw.1
            + u_norm * acceleration_rsw.2;
    }

    let u_h = h_vec / h_mag;
    let u_theta = u_h.cross(&u_r).normalized();

    u_r * acceleration_rsw.0 + u_theta * acceleration_rsw.1 + u_h * acceleration_rsw.2
}

pub fn gauss_variational_equations(
    elements: &OsculatingElements,
    perturbing_accel_rsw: Vector3,
    mu: GravitationalParameter,
) -> GaussVariationalRates {
    let mu_val = mu.value();
    let a = elements.semi_major_axis.value();
    let e = elements.eccentricity;
    let inc = elements.inclination.value();
    let omega = elements.argument_of_periapsis.value();
    let nu = elements.true_anomaly.value();

    if mu_val <= 0.0 || a <= 0.0 || e < 0.0 || e >= 1.0 || !a.is_finite() || !mu_val.is_finite() {
        return GaussVariationalRates::zero();
    }

    let p = a * (1.0 - e * e);
    let h = (mu_val * p).sqrt();
    let denom = 1.0 + e * nu.cos();

    if h <= 1e-12 || denom <= 1e-12 {
        return GaussVariationalRates::zero();
    }

    let r = p / denom;
    let u = omega + nu;

    let f_r = perturbing_accel_rsw.0;
    let f_theta = perturbing_accel_rsw.1;
    let f_h = perturbing_accel_rsw.2;

    let sin_nu = nu.sin();
    let cos_nu = nu.cos();
    let sin_u = u.sin();
    let cos_u = u.cos();
    let sin_inc = inc.sin();
    let cos_inc = inc.cos();

    let da_dt = (2.0 * a * a / h) * (e * sin_nu * f_r + (p / r) * f_theta);
    let de_dt = (1.0 / h) * (p * sin_nu * f_r + ((p + r) * cos_nu + r * e) * f_theta);
    let di_dt = (r * cos_u / h) * f_h;

    let draan_dt = if sin_inc.abs() > 1e-10 {
        (r * sin_u / (h * sin_inc)) * f_h
    } else {
        0.0
    };

    let domega_dt = if e > 1e-8 {
        let part1 = (1.0 / (h * e)) * (-p * cos_nu * f_r + (p + r) * sin_nu * f_theta);
        let part2 = if sin_inc.abs() > 1e-10 {
            (r * sin_u * cos_inc / (h * sin_inc)) * f_h
        } else {
            0.0
        };
        part1 - part2
    } else {
        0.0
    };

    let dnu_dt = (h / (r * r))
        + if e > 1e-8 {
            (1.0 / (h * e)) * (p * cos_nu * f_r - (p + r) * sin_nu * f_theta)
        } else {
            0.0
        };

    GaussVariationalRates::new(
        Speed::new(da_dt),
        de_dt,
        AngularVelocity::new(di_dt),
        AngularVelocity::new(draan_dt),
        AngularVelocity::new(domega_dt),
        AngularVelocity::new(dnu_dt),
    )
}

pub fn third_body_perturbation_acceleration(
    vehicle_pos_rel_primary: Vector3,
    third_body_pos_rel_primary: Vector3,
    third_body_mass: Mass,
) -> AccelerationVector {
    let m = third_body_mass.value();
    if m <= 0.0 || !m.is_finite() {
        return AccelerationVector::zero();
    }

    let r_v = vehicle_pos_rel_primary;
    let r_3 = third_body_pos_rel_primary;

    let d_v3 = r_3 - r_v;
    let dist_v3 = d_v3.magnitude();
    let dist_p3 = r_3.magnitude();

    if dist_v3 <= 1e-3 || dist_p3 <= 1e-3 || !dist_v3.is_finite() || !dist_p3.is_finite() {
        return AccelerationVector::zero();
    }

    let g_m = GRAVITATIONAL_CONSTANT * m;
    let term1 = d_v3 * (g_m / (dist_v3 * dist_v3 * dist_v3));
    let term2 = r_3 * (g_m / (dist_p3 * dist_p3 * dist_p3));
    let a = term1 - term2;

    if !a.0.is_finite() || !a.1.is_finite() || !a.2.is_finite() {
        AccelerationVector::zero()
    } else {
        AccelerationVector::from_raw(a)
    }
}

pub fn accumulated_third_body_perturbations(
    vehicle_pos_rel_primary: Vector3,
    third_bodies: &[(Vector3, Mass)],
) -> AccelerationVector {
    let mut total = Vector3::zero();
    for &(r_3, mass) in third_bodies {
        let a = third_body_perturbation_acceleration(vehicle_pos_rel_primary, r_3, mass);
        total = total + a.raw();
    }
    AccelerationVector::from_raw(total)
}

pub fn primary_gravitational_acceleration_with_j2(
    vehicle_position_inertial: Position,
    primary_position_inertial: Position,
    primary_orientation: Quaternion,
    primary_mu: GravitationalParameter,
    equatorial_radius: Length,
    j2: Option<f64>,
) -> AccelerationVector {
    let mu_val = primary_mu.value();
    if mu_val <= 0.0 || !mu_val.is_finite() {
        return AccelerationVector::zero();
    }

    let r_rel_inertial = vehicle_position_inertial.raw() - primary_position_inertial.raw();
    let dist = r_rel_inertial.magnitude();

    if dist <= 1e-3 || !dist.is_finite() {
        return AccelerationVector::zero();
    }

    let a_pm_raw = -r_rel_inertial * (mu_val / (dist * dist * dist));

    let a_j2_raw = match j2 {
        Some(j2_val) if j2_val != 0.0 && j2_val.is_finite() => {
            let r_bf = primary_orientation.inverse().rotate_vector(r_rel_inertial);
            let a_j2_bf = j2_gravitational_acceleration(
                primary_mu,
                equatorial_radius,
                j2_val,
                Position::from_raw(r_bf),
            );
            primary_orientation.rotate_vector(a_j2_bf.raw())
        }
        _ => Vector3::zero(),
    };

    AccelerationVector::from_raw(a_pm_raw + a_j2_raw)
}

pub fn angular_acceleration_from_torque(
    net_torque_body: TorqueVector,
    angular_velocity_body: AngularVelocityVector,
    inertia_tensor_body: &InertiaTensor,
) -> AngularAccelerationVector {
    let i_mat = inertia_tensor_body.matrix();
    let Some(i_inv) = inertia_tensor_body.inverse() else {
        return AngularAccelerationVector::zero();
    };

    let w = angular_velocity_body.raw();
    let h = i_mat.multiply_vector(w);
    let gyro_torque = w.cross(&h);
    let tau_eff = net_torque_body.raw() - gyro_torque;

    let alpha = i_inv.matrix().multiply_vector(tau_eff);
    if !alpha.0.is_finite() || !alpha.1.is_finite() || !alpha.2.is_finite() {
        AngularAccelerationVector::zero()
    } else {
        AngularAccelerationVector::from_components(alpha.0, alpha.1, alpha.2)
    }
}

pub fn compute_powered_flight_derivative(
    state: &RigidBodyState,
    total_mass: Mass,
    inertia_tensor: &InertiaTensor,
    gravity_acceleration: AccelerationVector,
    propulsion_forces: &VehiclePropulsionForces,
    aerodynamic_force: ForceVector,
) -> RigidBodyDerivative {
    let m = total_mass.value();
    let total_non_grav_force = propulsion_forces.net_world_force.raw() + aerodynamic_force.raw();

    let accel_non_grav = if m > 0.0 && m.is_finite() {
        total_non_grav_force / m
    } else {
        Vector3::zero()
    };

    let total_accel = gravity_acceleration.raw() + accel_non_grav;
    let alpha = angular_acceleration_from_torque(
        propulsion_forces.net_body_torque,
        state.angular_velocity,
        inertia_tensor,
    );

    RigidBodyDerivative::new(
        state.velocity,
        AccelerationVector::from_raw(total_accel),
        state.angular_velocity,
        alpha,
    )
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerturbedEnvironment {
    pub primary_position: Position,
    pub primary_orientation: Quaternion,
    pub primary_mu: GravitationalParameter,
    pub primary_equatorial_radius: Length,
    pub primary_j2: Option<f64>,
    pub third_bodies: Vec<(Vector3, Mass)>,
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
        Self {
            primary_position,
            primary_orientation,
            primary_mu,
            primary_equatorial_radius,
            primary_j2,
            third_bodies,
        }
    }

    pub fn gravitational_acceleration_at(&self, position: Position) -> AccelerationVector {
        let a_primary = primary_gravitational_acceleration_with_j2(
            position,
            self.primary_position,
            self.primary_orientation,
            self.primary_mu,
            self.primary_equatorial_radius,
            self.primary_j2,
        );

        let r_rel_primary = position.raw() - self.primary_position.raw();
        let a_third = accumulated_third_body_perturbations(r_rel_primary, &self.third_bodies);

        AccelerationVector::from_raw(a_primary.raw() + a_third.raw())
    }
}

pub fn evaluate_powered_flight_state_derivative(
    state: &RigidBodyState,
    total_mass: Mass,
    inertia_tensor: &InertiaTensor,
    environment: &PerturbedEnvironment,
    propulsion_forces: &VehiclePropulsionForces,
    aerodynamic_force: ForceVector,
) -> RigidBodyDerivative {
    let a_grav = environment.gravitational_acceleration_at(state.position);
    compute_powered_flight_derivative(
        state,
        total_mass,
        inertia_tensor,
        a_grav,
        propulsion_forces,
        aerodynamic_force,
    )
}