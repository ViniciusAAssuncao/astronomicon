use crate::constants::DEFAULT_SUTTON_GRAVES_CONSTANT;
use crate::math::aerothermodynamics::types::{
    AerocaptureVehicleProperties, AtmosphericModelParameters,
};
use astronomicon_core::units::constants::STANDARD_GRAVITY;
use astronomicon_core::units::{Length, Vector3};

pub fn atmospheric_derivatives(
    pos: Vector3,
    vel: Vector3,
    atm_params: &AtmosphericModelParameters,
    vehicle_props: &AerocaptureVehicleProperties,
) -> (Vector3, Vector3, f64, f64, f64, f64) {
    let r = pos.magnitude();
    let planet_r = atm_params.planet_radius.value();
    let h = r - planet_r;
    let mu = atm_params.gravitational_parameter.value();
    let m = vehicle_props.mass.value();
    let cd = vehicle_props.drag_coefficient;
    let a = vehicle_props.reference_area_m2;
    let ld = vehicle_props.lift_to_drag_ratio;
    let rn = vehicle_props.nose_radius.value();
    let k_sg = DEFAULT_SUTTON_GRAVES_CONSTANT;

    if r <= 1.0 || m <= 0.0 {
        return (vel, Vector3::zero(), 0.0, 0.0, 0.0, 0.0);
    }

    let rho = atm_params.density_at_altitude(Length::new(h)).value();

    let v_rel = if let Some(omega_p) = atm_params.planet_rotation_rate {
        let omega_vec = Vector3::new(0.0, 0.0, omega_p.value());
        vel - omega_vec.cross(&pos)
    } else {
        vel
    };

    let v_rel_mag = v_rel.magnitude();
    let q = 0.5 * rho * v_rel_mag * v_rel_mag;

    let stag_flux = if rn > 0.0 && rho > 0.0 && v_rel_mag > 0.0 {
        k_sg * (rho / rn).sqrt() * v_rel_mag.powi(3)
    } else {
        0.0
    };

    let drag_force = if v_rel_mag > 1e-6 {
        -v_rel * ((q * cd * a) / v_rel_mag)
    } else {
        Vector3::zero()
    };

    let lift_force = if ld.abs() > 1e-6 && v_rel_mag > 1e-6 {
        let u_v = v_rel / v_rel_mag;
        let h_orbit = pos.cross(&v_rel);
        let u_h = if h_orbit.magnitude() > 1e-12 {
            h_orbit.normalized()
        } else {
            u_v.any_perpendicular()
        };
        let u_lift = u_h.cross(&u_v).normalized();
        u_lift * (q * (cd * ld) * a)
    } else {
        Vector3::zero()
    };

    let aero_force = drag_force + lift_force;
    let a_aero = aero_force / m;
    let g_load = a_aero.magnitude() / STANDARD_GRAVITY;

    let a_grav = -pos * (mu / (r * r * r));
    let a_total = a_grav + a_aero;

    (vel, a_total, q, stag_flux, g_load, rho)
}