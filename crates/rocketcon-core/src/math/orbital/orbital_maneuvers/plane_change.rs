use astronomicon_core::units::{Angle, Speed, VelocityVector};

pub fn plane_change_delta_v(
    speed: Speed,
    flight_path_angle: Angle,
    inclination_change: Angle,
) -> Speed {
    let v = speed.value();
    let gamma = flight_path_angle.value();
    let di = inclination_change.value();

    if v <= 0.0 || !v.is_finite() || !gamma.is_finite() || !di.is_finite() {
        return Speed::new(0.0);
    }

    let dv = 2.0 * v * gamma.cos() * (0.5 * di).sin().abs();
    Speed::new(dv)
}

pub fn node_plane_change_delta_v(
    velocity: VelocityVector,
    inclination_change: Angle,
) -> Speed {
    let v = velocity.magnitude().value();
    let di = inclination_change.value();

    if v <= 0.0 || !v.is_finite() || !di.is_finite() {
        return Speed::new(0.0);
    }

    let dv = 2.0 * v * (0.5 * di).sin().abs();
    Speed::new(dv)
}

pub fn combined_plane_and_altitude_change_delta_v(
    v_initial: Speed,
    v_final: Speed,
    plane_change_angle: Angle,
) -> Speed {
    let v1 = v_initial.value();
    let v2 = v_final.value();
    let theta = plane_change_angle.value();

    if v1 < 0.0 || v2 < 0.0 || !v1.is_finite() || !v2.is_finite() || !theta.is_finite() {
        return Speed::new(0.0);
    }

    let dv_sq = v1 * v1 + v2 * v2 - 2.0 * v1 * v2 * theta.cos();
    Speed::new(dv_sq.max(0.0).sqrt())
}

pub fn optimal_combined_plane_change_angle(
    v_initial: Speed,
    v_final: Speed,
    total_plane_change: Angle,
) -> Angle {
    let v1 = v_initial.value();
    let v2 = v_final.value();
    let di = total_plane_change.value();

    if v1 <= 0.0 || v2 <= 0.0 || !v1.is_finite() || !v2.is_finite() || !di.is_finite() {
        return Angle::new(0.0);
    }

    let r = (v1 / v2).powi(3);
    let tan_theta1 = di.sin() / (di.cos() + r);
    Angle::new(tan_theta1.atan().clamp(0.0, di.abs()))
}