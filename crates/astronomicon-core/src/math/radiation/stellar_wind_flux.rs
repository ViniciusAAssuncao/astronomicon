use crate::units::{Density, Irradiance, Speed};

pub fn stellar_particle_flux(wind_density: Density, terminal_speed: Speed) -> Irradiance {
    let rho = wind_density.value();
    let v = terminal_speed.value();

    if rho <= 0.0 || v <= 0.0 || !rho.is_finite() || !v.is_finite() {
        return Irradiance::new(0.0);
    }

    let flux = 0.5 * rho * v * v * v;
    if !flux.is_finite() || flux < 0.0 {
        Irradiance::new(0.0)
    } else {
        Irradiance::new(flux)
    }
}
