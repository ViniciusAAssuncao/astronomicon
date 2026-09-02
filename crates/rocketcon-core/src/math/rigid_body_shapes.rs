use astronomicon_core::units::{ InertiaTensor, Length, Mass };

pub fn solid_cylinder_inertia_tensor(mass: Mass, radius: Length, length: Length) -> InertiaTensor {
    let m = mass.value();
    let r = radius.value();
    let l = length.value();

    if m <= 0.0 || r <= 0.0 || l <= 0.0 || !m.is_finite() || !r.is_finite() || !l.is_finite() {
        return InertiaTensor::zero();
    }

    let i_axial = 0.5 * m * r * r;
    let i_transversal = (1.0 / 12.0) * m * (3.0 * r * r + l * l);

    InertiaTensor::principal_diagonal(i_transversal, i_transversal, i_axial)
}

pub fn thin_shell_cylinder_inertia_tensor(
    mass: Mass,
    radius: Length,
    length: Length
) -> InertiaTensor {
    let m = mass.value();
    let r = radius.value();
    let l = length.value();

    if m <= 0.0 || r <= 0.0 || l <= 0.0 || !m.is_finite() || !r.is_finite() || !l.is_finite() {
        return InertiaTensor::zero();
    }

    let i_axial = m * r * r;
    let i_transversal = (m * (6.0 * r * r + l * l)) / 12.0;

    InertiaTensor::principal_diagonal(i_transversal, i_transversal, i_axial)
}
