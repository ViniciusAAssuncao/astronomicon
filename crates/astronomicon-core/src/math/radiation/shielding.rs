use crate::units::constants::GALACTIC_COSMIC_RAY_BACKGROUND_DOSE;
use crate::units::{Angle, Energy, Length, MagneticDipoleMoment, MagneticRigidity, RadiationDose};

pub fn galactic_cosmic_ray_background() -> RadiationDose {
    RadiationDose::new(GALACTIC_COSMIC_RAY_BACKGROUND_DOSE)
}

pub fn cutoff_rigidity(
    dipole_moment: MagneticDipoleMoment,
    radius: Length,
    latitude: Angle,
) -> MagneticRigidity {
    let m = dipole_moment.value();
    let r = radius.value();
    let lat = latitude.value();

    if m <= 0.0 || r <= 0.0 || !m.is_finite() || !r.is_finite() || !lat.is_finite() {
        return MagneticRigidity::new(0.0);
    }

    let cos_lat = lat.cos();
    let cos4 = cos_lat * cos_lat * cos_lat * cos_lat;
    let rigidity = (m / (4.0 * r * r)) * cos4;

    if !rigidity.is_finite() || rigidity < 0.0 {
        MagneticRigidity::new(0.0)
    } else {
        MagneticRigidity::new(rigidity)
    }
}

pub fn magnetosphere_shielding_factor(
    cutoff_rigidity: MagneticRigidity,
    particle_kinetic_energy: Energy,
) -> f64 {
    let rc = cutoff_rigidity.value();
    let e = particle_kinetic_energy.value();

    if e <= 0.0 || !e.is_finite() {
        return 0.0;
    }

    if rc <= 0.0 || !rc.is_finite() {
        return 1.0;
    }

    let ratio = rc / e;
    let transmissivity = 1.0 / (1.0 + ratio.powi(4));

    transmissivity.clamp(0.0, 1.0)
}
