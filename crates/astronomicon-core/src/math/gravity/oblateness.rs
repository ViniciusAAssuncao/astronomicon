use crate::units::{AccelerationVector, GravitationalParameter, Length, Position};

pub fn j2_gravitational_acceleration(
    mu: GravitationalParameter,
    equatorial_radius: Length,
    j2: f64,
    position_relative_to_body_fixed_frame: Position,
) -> AccelerationVector {
    let mu_val = mu.value();
    let r_eq = equatorial_radius.value();

    if mu_val <= 0.0
        || !mu_val.is_finite()
        || r_eq <= 0.0
        || !r_eq.is_finite()
        || !j2.is_finite()
        || j2 == 0.0
    {
        return AccelerationVector::zero();
    }

    let pos = position_relative_to_body_fixed_frame.raw();
    let x = pos.0;
    let y = pos.1;
    let z = pos.2;

    let r_sq = x * x + y * y + z * z;
    if r_sq <= 1e-12 || !r_sq.is_finite() {
        return AccelerationVector::zero();
    }

    let r = r_sq.sqrt();
    let r5 = r_sq * r_sq * r;
    let k = -1.5 * mu_val * j2 * r_eq * r_eq / r5;
    let z_ratio_sq = (z * z) / r_sq;

    let ax = k * x * (1.0 - 5.0 * z_ratio_sq);
    let ay = k * y * (1.0 - 5.0 * z_ratio_sq);
    let az = k * z * (3.0 - 5.0 * z_ratio_sq);

    if !ax.is_finite() || !ay.is_finite() || !az.is_finite() {
        AccelerationVector::zero()
    } else {
        AccelerationVector::from_components(ax, ay, az)
    }
}