use astronomicon_core::units::{
    Angle, GravitationalParameter, Length, Position, Speed, Vector3, VelocityVector,
};

pub fn vacuum_periapsis_from_entry_angle(
    entry_radius: Length,
    entry_speed: Speed,
    flight_path_angle: Angle,
    mu: GravitationalParameter,
) -> Length {
    let r_e = entry_radius.value();
    let v_e = entry_speed.value();
    let gamma = flight_path_angle.value();
    let mu_val = mu.value();

    if r_e <= 0.0 || v_e <= 0.0 || mu_val <= 0.0 || !r_e.is_finite() || !v_e.is_finite() {
        return Length::new(0.0);
    }

    let h = r_e * v_e * gamma.cos().abs();
    let energy = 0.5 * v_e * v_e - mu_val / r_e;
    let e_sq = 1.0 + (2.0 * energy * h * h) / (mu_val * mu_val);
    let e = e_sq.max(0.0).sqrt();

    let p = (h * h) / mu_val;
    let rp = p / (1.0 + e);
    Length::new(rp.max(0.0))
}

pub fn entry_angle_from_vacuum_periapsis(
    entry_radius: Length,
    entry_speed: Speed,
    vacuum_periapsis_radius: Length,
    mu: GravitationalParameter,
) -> Option<Angle> {
    let r_e = entry_radius.value();
    let v_e = entry_speed.value();
    let rp = vacuum_periapsis_radius.value();
    let mu_val = mu.value();

    if r_e <= 0.0
        || v_e <= 0.0
        || rp <= 0.0
        || rp >= r_e
        || mu_val <= 0.0
        || !r_e.is_finite()
        || !v_e.is_finite()
        || !rp.is_finite()
    {
        return None;
    }

    let energy = 0.5 * v_e * v_e - mu_val / r_e;
    let vp = (2.0 * (energy + mu_val / rp)).max(0.0).sqrt();
    let h = rp * vp;

    let cos_gamma = (h / (r_e * v_e)).clamp(0.0, 1.0);
    let gamma = -cos_gamma.acos();
    Some(Angle::new(gamma))
}

pub fn state_from_entry_parameters(
    entry_radius: Length,
    entry_speed: Speed,
    flight_path_angle: Angle,
    inclination: Angle,
    azimuth: Angle,
) -> (Position, VelocityVector) {
    let r = entry_radius.value();
    let v = entry_speed.value();
    let gamma = flight_path_angle.value();
    let inc = inclination.value();
    let az = azimuth.value();

    let r_pos = Vector3::new(r * inc.cos(), 0.0, r * inc.sin());

    let v_rad = v * gamma.sin();
    let v_horiz = v * gamma.cos();

    let v_x = v_rad * inc.cos() - v_horiz * inc.sin() * az.cos();
    let v_y = v_horiz * az.sin();
    let v_z = v_rad * inc.sin() + v_horiz * inc.cos() * az.cos();

    let v_vec = Vector3::new(v_x, v_y, v_z);

    (Position::from_raw(r_pos), VelocityVector::from_raw(v_vec))
}