use astronomicon_core::domain::OrbitalElements;

const SAMPLES: usize = 128;

pub fn catalog_orbit_points(
    elements: OrbitalElements,
    parent_position_m: [f64; 3],
    origin_position_m: [f64; 3],
) -> Vec<[f64; 3]> {
    let a = elements.semi_major_axis().value();
    let e = elements.eccentricity();
    if !a.is_finite() || a <= 0.0 || !e.is_finite() || !(0.0..1.0).contains(&e) {
        return Vec::new();
    }
    let (sin_node, cos_node) = elements.longitude_of_ascending_node().value().sin_cos();
    let (sin_i, cos_i) = elements.inclination().value().sin_cos();
    let (sin_w, cos_w) = elements.argument_of_periapsis().value().sin_cos();
    let axis_x = [
        cos_node * cos_w - sin_node * sin_w * cos_i,
        sin_node * cos_w + cos_node * sin_w * cos_i,
        sin_w * sin_i,
    ];
    let axis_y = [
        -cos_node * sin_w - sin_node * cos_w * cos_i,
        -sin_node * sin_w + cos_node * cos_w * cos_i,
        cos_w * sin_i,
    ];
    let b = a * (1.0 - e * e).sqrt();
    let mut points = Vec::with_capacity(SAMPLES + 1);
    for step in 0..=SAMPLES {
        let anomaly = std::f64::consts::TAU * step as f64 / SAMPLES as f64;
        let (s, c) = anomaly.sin_cos();
        let x = a * (c - e);
        let y = b * s;
        let point = std::array::from_fn(|index| {
            parent_position_m[index] - origin_position_m[index]
                + x * axis_x[index]
                + y * axis_y[index]
        });
        if point.iter().any(|coordinate| !coordinate.is_finite()) {
            return Vec::new();
        }
        points.push(point);
    }
    points
}
