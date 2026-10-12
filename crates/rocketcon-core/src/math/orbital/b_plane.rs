use crate::error::{ RocketDomainError, RocketDomainResult };
use astronomicon_core::units::{
    Angle,
    GravitationalParameter,
    Length,
    Position,
    Speed,
    Vector3,
    VelocityVector,
};
use serde::{ Deserialize, Serialize };

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BPlaneBasis {
    pub s: Vector3,
    pub t: Vector3,
    pub r: Vector3,
}

impl BPlaneBasis {
    pub fn new(s: Vector3, t: Vector3, r: Vector3) -> Self {
        Self { s, t, r }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BPlaneCoordinates {
    pub b_r: Length,
    pub b_t: Length,
    pub magnitude: Length,
    pub clock_angle: Angle,
}

impl BPlaneCoordinates {
    pub fn new(b_r: Length, b_t: Length, magnitude: Length, clock_angle: Angle) -> Self {
        Self {
            b_r,
            b_t,
            magnitude,
            clock_angle,
        }
    }

    pub fn from_components(b_r: Length, b_t: Length) -> Self {
        let br = b_r.value();
        let bt = b_t.value();
        let mag = (br * br + bt * bt).sqrt();
        let theta = br.atan2(bt);
        Self {
            b_r,
            b_t,
            magnitude: Length::new(mag),
            clock_angle: Angle::new(theta),
        }
    }
}

pub fn b_plane_basis(v_infinity: Vector3, reference_pole: Vector3) -> BPlaneBasis {
    let s = v_infinity.normalized();
    let pole = reference_pole.normalized();
    let s_cross_pole = s.cross(&pole);
    let t = if s_cross_pole.magnitude() > 1e-12 {
        s_cross_pole.normalized()
    } else {
        s.any_perpendicular()
    };
    let r = s.cross(&t).normalized();
    BPlaneBasis::new(s, t, r)
}

pub fn impact_parameter_from_periapsis(
    periapsis_radius: Length,
    v_infinity: Speed,
    mu: GravitationalParameter
) -> Length {
    let rp = periapsis_radius.value();
    let v_inf = v_infinity.value();
    let mu_val = mu.value();

    if
        rp <= 0.0 ||
        v_inf <= 0.0 ||
        mu_val <= 0.0 ||
        !rp.is_finite() ||
        !v_inf.is_finite() ||
        !mu_val.is_finite()
    {
        return Length::new(0.0);
    }

    let term = 1.0 + (2.0 * mu_val) / (rp * v_inf * v_inf);
    Length::new(rp * term.sqrt())
}

pub fn periapsis_from_impact_parameter(
    impact_parameter: Length,
    v_infinity: Speed,
    mu: GravitationalParameter
) -> Length {
    let b = impact_parameter.value();
    let v_inf = v_infinity.value();
    let mu_val = mu.value();

    if
        b <= 0.0 ||
        v_inf <= 0.0 ||
        mu_val <= 0.0 ||
        !b.is_finite() ||
        !v_inf.is_finite() ||
        !mu_val.is_finite()
    {
        return Length::new(0.0);
    }

    let v_sq = v_inf * v_inf;
    let ratio = (b * v_sq) / mu_val;
    let rp = (mu_val / v_sq) * ((1.0 + ratio * ratio).sqrt() - 1.0);
    Length::new(rp.max(0.0))
}

pub fn deflection_angle(
    periapsis_radius: Length,
    v_infinity: Speed,
    mu: GravitationalParameter
) -> Angle {
    let rp = periapsis_radius.value();
    let v_inf = v_infinity.value();
    let mu_val = mu.value();

    if
        rp <= 0.0 ||
        v_inf <= 0.0 ||
        mu_val <= 0.0 ||
        !rp.is_finite() ||
        !v_inf.is_finite() ||
        !mu_val.is_finite()
    {
        return Angle::new(0.0);
    }

    let e = 1.0 + (rp * v_inf * v_inf) / mu_val;
    let sin_half_delta = (1.0 / e).clamp(-1.0, 1.0);
    Angle::new(2.0 * sin_half_delta.asin())
}

pub fn deflection_angle_from_impact_parameter(
    impact_parameter: Length,
    v_infinity: Speed,
    mu: GravitationalParameter
) -> Angle {
    let b = impact_parameter.value();
    let v_inf = v_infinity.value();
    let mu_val = mu.value();

    if
        b <= 0.0 ||
        v_inf <= 0.0 ||
        mu_val <= 0.0 ||
        !b.is_finite() ||
        !v_inf.is_finite() ||
        !mu_val.is_finite()
    {
        return Angle::new(0.0);
    }

    let tan_half_delta = mu_val / (b * v_inf * v_inf);
    Angle::new(2.0 * tan_half_delta.atan())
}

pub fn b_vector(coordinates: &BPlaneCoordinates, basis: &BPlaneBasis) -> Vector3 {
    basis.t * coordinates.b_t.value() + basis.r * coordinates.b_r.value()
}

pub fn b_vector_from_periapsis_and_clock_angle(
    periapsis_radius: Length,
    clock_angle: Angle,
    v_infinity: Speed,
    mu: GravitationalParameter,
    basis: &BPlaneBasis
) -> Vector3 {
    let b = impact_parameter_from_periapsis(periapsis_radius, v_infinity, mu).value();
    let theta = clock_angle.value();
    let b_t = b * theta.cos();
    let b_r = b * theta.sin();
    basis.t * b_t + basis.r * b_r
}

pub fn outgoing_asymptote_from_b_plane(
    s_incoming: Vector3,
    b_vec: Vector3,
    deflection_angle: Angle
) -> Vector3 {
    let s_hat = s_incoming.normalized();
    let h_cross = s_hat.cross(&b_vec);
    let h_hat = if h_cross.magnitude() > 1e-12 {
        h_cross.normalized()
    } else {
        s_hat.any_perpendicular()
    };

    let delta = deflection_angle.value();
    let normal_in_plane = h_hat.cross(&s_hat).normalized();
    (s_hat * delta.cos() + normal_in_plane * delta.sin()).normalized()
}

pub fn b_plane_parameters_from_state(
    position: Position,
    velocity: VelocityVector,
    mu: GravitationalParameter,
    reference_pole: Vector3
) -> RocketDomainResult<BPlaneCoordinates> {
    let r_vec = position.raw();
    let v_vec = velocity.raw();
    let mu_val = mu.value();

    let r = r_vec.magnitude();
    let v_sq = v_vec.dot(&v_vec);

    if r <= 1e-6 || mu_val <= 0.0 || !r.is_finite() || !v_sq.is_finite() {
        return Err(RocketDomainError::InvalidInvariant {
            field: "state".to_string(),
            reason: "invalid position or gravitational parameter".to_string(),
        });
    }

    let energy = 0.5 * v_sq - mu_val / r;
    if energy <= 0.0 {
        return Err(RocketDomainError::InvalidInvariant {
            field: "energy".to_string(),
            reason: "orbit must be hyperbolic to calculate B-plane coordinates".to_string(),
        });
    }

    let v_inf = (2.0 * energy).sqrt();
    let h_vec = r_vec.cross(&v_vec);
    let h = h_vec.magnitude();

    if h <= 1e-12 {
        return Err(RocketDomainError::InvalidInvariant {
            field: "angular_momentum".to_string(),
            reason: "trajectory is purely radial".to_string(),
        });
    }

    let e_vec = v_vec.cross(&h_vec) / mu_val - r_vec / r;
    let e = e_vec.magnitude();
    if e <= 1.0 {
        return Err(RocketDomainError::InvalidInvariant {
            field: "eccentricity".to_string(),
            reason: "eccentricity must be greater than 1".to_string(),
        });
    }

    let p_hat = e_vec / e;
    let h_hat = h_vec / h;
    let q_hat = h_hat.cross(&p_hat).normalized();

    let nu_inf = -(-1.0 / e).clamp(-1.0, 1.0).acos();
    let s_hat = p_hat * nu_inf.cos() + q_hat * nu_inf.sin();

    let a = -mu_val / (v_inf * v_inf);
    let b_mag = -a * (e * e - 1.0).sqrt();

    let b_vec = s_hat.cross(&h_hat).normalized() * b_mag;
    let basis = b_plane_basis(s_hat, reference_pole);

    let b_t = b_vec.dot(&basis.t);
    let b_r = b_vec.dot(&basis.r);

    Ok(BPlaneCoordinates::from_components(Length::new(b_r), Length::new(b_t)))
}
