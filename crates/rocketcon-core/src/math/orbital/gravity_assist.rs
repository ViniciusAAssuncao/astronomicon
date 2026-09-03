use crate::error::{ RocketDomainError, RocketDomainResult };
use crate::math::orbital::b_plane::{ b_plane_basis, BPlaneBasis, BPlaneCoordinates };
use astronomicon_core::units::{
    Angle,
    Duration,
    GravitationalParameter,
    Length,
    Speed,
    Vector3,
    VelocityVector,
};
use serde::{ Deserialize, Serialize };
use uuid::Uuid;

pub fn tisserand_parameter(
    semi_major_axis: Length,
    eccentricity: f64,
    inclination: Angle,
    planet_semi_major_axis: Length
) -> f64 {
    let a = semi_major_axis.value();
    let ap = planet_semi_major_axis.value();
    let e = eccentricity;
    let inc = inclination.value();

    if a <= 0.0 || ap <= 0.0 || !a.is_finite() || !ap.is_finite() {
        return 0.0;
    }

    let e_term = (1.0 - e * e).max(0.0).sqrt();
    ap / a + 2.0 * (a / ap).sqrt() * e_term * inc.cos()
}

pub fn tisserand_v_infinity(tisserand_parameter: f64, planet_orbital_speed: Speed) -> Speed {
    let vp = planet_orbital_speed.value();
    let t = tisserand_parameter;
    if vp <= 0.0 || !vp.is_finite() || !t.is_finite() {
        return Speed::new(0.0);
    }
    let ratio = (3.0 - t).max(0.0);
    Speed::new(vp * ratio.sqrt())
}

pub fn tisserand_semi_major_axis_estimate(
    tisserand_parameter: f64,
    eccentricity: f64,
    inclination: Angle,
    planet_semi_major_axis: Length
) -> Option<Length> {
    let ap = planet_semi_major_axis.value();
    let t = tisserand_parameter;
    let e = eccentricity;
    let inc = inclination.value();

    if ap <= 0.0 || e < 0.0 || e >= 1.0 || !ap.is_finite() || !t.is_finite() {
        return None;
    }

    let k = 2.0 * (1.0 - e * e).max(0.0).sqrt() * inc.cos();
    let a_guess = ap;
    let mut x = (a_guess / ap).sqrt();

    for _ in 0..30 {
        let f = 1.0 / (x * x) + k * x - t;
        let df = -2.0 / (x * x * x) + k;
        if df.abs() < 1e-12 {
            break;
        }
        let dx = f / df;
        x -= dx;
        if dx.abs() < 1e-10 {
            let a = x * x * ap;
            if a > 0.0 && a.is_finite() {
                return Some(Length::new(a));
            }
        }
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FlybyDeflection {
    pub v_infinity_in: Speed,
    pub v_infinity_out: Speed,
    pub turning_angle: Angle,
    pub max_turning_angle: Angle,
    pub periapsis_radius: Length,
    pub periapsis_altitude: Length,
    pub periapsis_velocity_in: Speed,
    pub periapsis_velocity_out: Speed,
    pub delta_v_periapsis: Speed,
    pub is_unpowered: bool,
}

pub fn solve_gravity_assist_deflection(
    v_inf_in: VelocityVector,
    v_inf_out: VelocityVector,
    planet_radius: Length,
    min_altitude: Length,
    mu: GravitationalParameter
) -> RocketDomainResult<FlybyDeflection> {
    let v1 = v_inf_in.raw();
    let v2 = v_inf_out.raw();
    let v1_mag = v1.magnitude();
    let v2_mag = v2.magnitude();
    let mu_val = mu.value();
    let r_planet = planet_radius.value();
    let h_min = min_altitude.value();
    let rp_min = r_planet + h_min;

    if v1_mag <= 1e-6 || v2_mag <= 1e-6 || mu_val <= 0.0 || rp_min <= 0.0 {
        return Err(RocketDomainError::InvalidInvariant {
            field: "gravity_assist_deflection".to_string(),
            reason: "velocities, gravitational parameter, and periapsis radius must be positive".to_string(),
        });
    }

    let cos_delta = (v1.dot(&v2) / (v1_mag * v2_mag)).clamp(-1.0, 1.0);
    let delta = cos_delta.acos();

    let calc_delta_max = |r: f64, v_in: f64, v_out: f64| -> f64 {
        let sin1 = (1.0 / (1.0 + (r * v_in * v_in) / mu_val)).clamp(0.0, 1.0);
        let sin2 = (1.0 / (1.0 + (r * v_out * v_out) / mu_val)).clamp(0.0, 1.0);
        sin1.asin() + sin2.asin()
    };

    let delta_max = calc_delta_max(rp_min, v1_mag, v2_mag);

    let (rp, delta_v, is_unpowered) = if delta <= delta_max {
        let mut r_low = rp_min;
        let mut r_high = rp_min * 2.0;

        while calc_delta_max(r_high, v1_mag, v2_mag) > delta && r_high < 1e9 * rp_min {
            r_high *= 2.0;
        }

        for _ in 0..60 {
            let r_mid = 0.5 * (r_low + r_high);
            let d_mid = calc_delta_max(r_mid, v1_mag, v2_mag);
            if d_mid > delta {
                r_low = r_mid;
            } else {
                r_high = r_mid;
            }
        }

        let solved_rp = 0.5 * (r_low + r_high);
        let vp1 = (v1_mag * v1_mag + (2.0 * mu_val) / solved_rp).sqrt();
        let vp2 = (v2_mag * v2_mag + (2.0 * mu_val) / solved_rp).sqrt();
        let dv = (vp2 - vp1).abs();
        let unpowered = dv < 1e-4 && (v1_mag - v2_mag).abs() < 1e-4;
        (solved_rp, dv, unpowered)
    } else {
        let vp1 = (v1_mag * v1_mag + (2.0 * mu_val) / rp_min).sqrt();
        let vp2 = (v2_mag * v2_mag + (2.0 * mu_val) / rp_min).sqrt();
        let eta = delta - delta_max;
        let dv_sq = vp1 * vp1 + vp2 * vp2 - 2.0 * vp1 * vp2 * eta.cos();
        let dv = dv_sq.max(0.0).sqrt();
        (rp_min, dv, false)
    };

    let vp_in = (v1_mag * v1_mag + (2.0 * mu_val) / rp).sqrt();
    let vp_out = (v2_mag * v2_mag + (2.0 * mu_val) / rp).sqrt();

    Ok(FlybyDeflection {
        v_infinity_in: Speed::new(v1_mag),
        v_infinity_out: Speed::new(v2_mag),
        turning_angle: Angle::new(delta),
        max_turning_angle: Angle::new(delta_max),
        periapsis_radius: Length::new(rp),
        periapsis_altitude: Length::new(rp - r_planet),
        periapsis_velocity_in: Speed::new(vp_in),
        periapsis_velocity_out: Speed::new(vp_out),
        delta_v_periapsis: Speed::new(delta_v),
        is_unpowered,
    })
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GravityAssistFlyby {
    pub body_id: Uuid,
    pub v_infinity_in: VelocityVector,
    pub v_infinity_out: VelocityVector,
    pub deflection: FlybyDeflection,
    pub b_plane: BPlaneCoordinates,
    pub b_plane_basis: BPlaneBasis,
    pub incoming_heliocentric_velocity: VelocityVector,
    pub outgoing_heliocentric_velocity: VelocityVector,
    pub planet_velocity: VelocityVector,
}

pub fn solve_gravity_assist_flyby(
    body_id: Uuid,
    v_in_heliocentric: VelocityVector,
    v_out_heliocentric: VelocityVector,
    planet_velocity: VelocityVector,
    planet_radius: Length,
    min_flyby_altitude: Length,
    mu: GravitationalParameter,
    reference_pole: Vector3
) -> RocketDomainResult<GravityAssistFlyby> {
    let v_inf_in = VelocityVector::from_raw(v_in_heliocentric.raw() - planet_velocity.raw());
    let v_inf_out = VelocityVector::from_raw(v_out_heliocentric.raw() - planet_velocity.raw());

    let deflection = solve_gravity_assist_deflection(
        v_inf_in,
        v_inf_out,
        planet_radius,
        min_flyby_altitude,
        mu
    )?;

    let rp = deflection.periapsis_radius.value();
    let v_inf_mag = deflection.v_infinity_in.value();
    let mu_val = mu.value();
    let b_mag = rp * (1.0 + (2.0 * mu_val) / (rp * v_inf_mag * v_inf_mag)).sqrt();

    let s_in = v_inf_in.raw().normalized();
    let s_out = v_inf_out.raw().normalized();
    let cross_s = s_in.cross(&s_out);
    let h_enc = if cross_s.magnitude() > 1e-10 {
        cross_s.normalized()
    } else {
        s_in.any_perpendicular()
    };

    let b_vec = s_in.cross(&h_enc).normalized() * b_mag;
    let basis = b_plane_basis(s_in, reference_pole);

    let b_t = b_vec.dot(&basis.t);
    let b_r = b_vec.dot(&basis.r);
    let b_coords = BPlaneCoordinates::from_components(Length::new(b_r), Length::new(b_t));

    Ok(GravityAssistFlyby {
        body_id,
        v_infinity_in: v_inf_in,
        v_infinity_out: v_inf_out,
        deflection,
        b_plane: b_coords,
        b_plane_basis: basis,
        incoming_heliocentric_velocity: v_in_heliocentric,
        outgoing_heliocentric_velocity: v_out_heliocentric,
        planet_velocity,
    })
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlybyWaypointPlan {
    pub body_id: Uuid,
    pub encounter_epoch: Duration,
    pub flyby: GravityAssistFlyby,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GravityAssistTourPlan {
    pub departure_body_id: Uuid,
    pub destination_body_id: Uuid,
    pub departure_epoch: Duration,
    pub total_duration: Duration,
    pub departure_delta_v: Speed,
    pub arrival_delta_v: Speed,
    pub total_mission_delta_v: Speed,
    pub waypoints: Vec<FlybyWaypointPlan>,
}
