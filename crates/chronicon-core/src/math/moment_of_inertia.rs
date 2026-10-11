use astronomicon_core::units::{Density, Length, Mass, MomentOfInertia};
use std::f64::consts::PI;

pub fn solid_sphere_moment_of_inertia(mass: Mass, radius: Length) -> MomentOfInertia {
    let m = mass.value();
    let r = radius.value();

    if m <= 0.0 || r <= 0.0 || !m.is_finite() || !r.is_finite() {
        return MomentOfInertia::new(0.0);
    }

    MomentOfInertia::new(0.4 * m * r * r)
}

pub fn spherical_shell_moment_of_inertia(
    mass: Mass,
    inner_radius: Length,
    outer_radius: Length,
) -> MomentOfInertia {
    let m = mass.value();
    let r_in = inner_radius.value();
    let r_out = outer_radius.value();

    if m <= 0.0 || r_out <= 0.0 || !m.is_finite() || !r_out.is_finite() {
        return MomentOfInertia::new(0.0);
    }

    if r_in <= 0.0 || !r_in.is_finite() {
        return solid_sphere_moment_of_inertia(mass, outer_radius);
    }

    if r_in >= r_out {
        return solid_sphere_moment_of_inertia(mass, outer_radius);
    }

    let r_out3 = r_out.powi(3);
    let r_in3 = r_in.powi(3);
    let r_out5 = r_out.powi(5);
    let r_in5 = r_in.powi(5);

    let denom = r_out3 - r_in3;
    if denom <= 0.0 || !denom.is_finite() {
        return solid_sphere_moment_of_inertia(mass, outer_radius);
    }

    let num = r_out5 - r_in5;
    let factor = num / denom;

    MomentOfInertia::new(0.4 * m * factor)
}

pub fn core_radius_from_mass_fraction_and_density(
    total_mass: Mass,
    core_mass_fraction: f64,
    core_density: Density,
) -> Length {
    let m = total_mass.value();
    let cmf = core_mass_fraction.clamp(0.0, 1.0);
    let rho_c = core_density.value();

    if m <= 0.0 || cmf <= 0.0 || rho_c <= 0.0 || !m.is_finite() || !rho_c.is_finite() {
        return Length::new(0.0);
    }

    let m_core = m * cmf;
    let volume = m_core / rho_c;
    let r_c = (3.0 * volume / (4.0 * PI)).cbrt();

    if !r_c.is_finite() || r_c <= 0.0 {
        Length::new(0.0)
    } else {
        Length::new(r_c)
    }
}

pub fn core_radius_from_mantle_density(
    total_mass: Mass,
    outer_radius: Length,
    core_mass_fraction: f64,
    mantle_density: Density,
) -> Length {
    let m = total_mass.value();
    let r_out = outer_radius.value();
    let cmf = core_mass_fraction.clamp(0.0, 1.0);
    let rho_m = mantle_density.value();

    if cmf <= 0.0 {
        return Length::new(0.0);
    }

    if cmf >= 1.0 {
        return outer_radius;
    }

    if m <= 0.0 || r_out <= 0.0 || rho_m <= 0.0 || !m.is_finite() || !r_out.is_finite() || !rho_m.is_finite() {
        return Length::new(0.0);
    }

    let m_mantle = m * (1.0 - cmf);
    let v_mantle = m_mantle / rho_m;
    let v_total = (4.0 / 3.0) * PI * r_out.powi(3);
    let v_core = v_total - v_mantle;

    if v_core <= 0.0 || !v_core.is_finite() {
        return Length::new(0.0);
    }

    let r_core = (3.0 * v_core / (4.0 * PI)).cbrt();
    if !r_core.is_finite() || r_core <= 0.0 {
        Length::new(0.0)
    } else {
        Length::new(r_core.min(r_out))
    }
}

pub fn two_layer_polar_moment_of_inertia(
    total_mass: Mass,
    outer_radius: Length,
    core_mass_fraction: f64,
    core_radius: Length,
) -> MomentOfInertia {
    let m = total_mass.value();
    let r_out = outer_radius.value();
    let r_c = core_radius.value();
    let cmf = core_mass_fraction.clamp(0.0, 1.0);

    if m <= 0.0 || r_out <= 0.0 || !m.is_finite() || !r_out.is_finite() {
        return MomentOfInertia::new(0.0);
    }

    if cmf <= 0.0 || r_c <= 0.0 || !r_c.is_finite() {
        return solid_sphere_moment_of_inertia(total_mass, outer_radius);
    }

    if cmf >= 1.0 || r_c >= r_out {
        return solid_sphere_moment_of_inertia(total_mass, outer_radius);
    }

    let m_core = Mass::new(m * cmf);
    let m_mantle = Mass::new(m * (1.0 - cmf));

    let i_core = solid_sphere_moment_of_inertia(m_core, core_radius);
    let i_mantle = spherical_shell_moment_of_inertia(m_mantle, core_radius, outer_radius);

    MomentOfInertia::new(i_core.value() + i_mantle.value())
}

pub fn polar_moment_of_inertia_normalized_coefficient(
    total_mass: Mass,
    outer_radius: Length,
    moment_of_inertia: MomentOfInertia,
) -> f64 {
    let m = total_mass.value();
    let r = outer_radius.value();
    let c = moment_of_inertia.value();

    if m <= 0.0 || r <= 0.0 || c <= 0.0 || !m.is_finite() || !r.is_finite() || !c.is_finite() {
        return 0.4;
    }

    let norm = m * r * r;
    if norm <= 0.0 {
        0.4
    } else {
        c / norm
    }
}