use astronomicon_core::domain::Planet;
use astronomicon_core::math::lagrange::orbital_plane_normal;
use astronomicon_core::math::rotation_state::{
    planet_rotation_axis_direction, solstice_reference_direction,
};
use astronomicon_core::units::Vector3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbitalReferenceFrame {
    pub normal: Vector3,
    pub horizontal: Vector3,
    pub vertical: Vector3,
}

impl OrbitalReferenceFrame {
    pub fn inertial_xy() -> Self {
        Self {
            normal: Vector3::new(0.0, 0.0, 1.0),
            horizontal: Vector3::new(1.0, 0.0, 0.0),
            vertical: Vector3::new(0.0, 1.0, 0.0),
        }
    }

    pub fn from_pole(pole: Vector3) -> Option<Self> {
        let magnitude = pole.magnitude();
        if !magnitude.is_finite()
            || magnitude <= 1e-12
            || ![pole.0, pole.1, pole.2].iter().all(|v| v.is_finite())
        {
            return None;
        }
        let normal = pole / magnitude;
        let preferred = if normal.0.abs() < 0.9 {
            Vector3::new(1.0, 0.0, 0.0)
        } else {
            Vector3::new(0.0, 1.0, 0.0)
        };
        let projected = preferred - normal * normal.dot(&preferred);
        let horizontal = projected / projected.magnitude();
        let vertical = normal.cross(&horizontal);
        Some(Self {
            normal,
            horizontal,
            vertical,
        })
    }

    pub fn as_arrays(&self) -> ([f64; 3], [f64; 3], [f64; 3]) {
        let v = |p: Vector3| [p.0, p.1, p.2];
        (v(self.normal), v(self.horizontal), v(self.vertical))
    }
}

pub fn planet_equatorial_reference(planet: &Planet) -> Option<OrbitalReferenceFrame> {
    let elements = planet.orbital_elements()?;
    let obliquity = planet.obliquity()?;
    let solstice = planet.solstice_true_anomaly()?;
    let rotation_period = planet.rotation_period()?;
    if !rotation_period.value().is_finite() || rotation_period.value() <= 0.0 {
        return None;
    }
    let normal = orbital_plane_normal(&elements);
    let solstice_direction = solstice_reference_direction(&elements, solstice);
    let pole = planet_rotation_axis_direction(normal, obliquity, solstice_direction);
    OrbitalReferenceFrame::from_pole(pole)
}