use crate::math::thermodynamics::MatterState;

pub fn dynamic_surface_albedo(
    base_albedo: f64,
    hydrosphere_state: MatterState,
    surface_coverage_fraction: f64,
    liquid_albedo: f64,
    ice_albedo: f64,
) -> f64 {
    let base = base_albedo.clamp(0.0, 1.0);
    let cov = surface_coverage_fraction.clamp(0.0, 1.0);

    if cov <= 0.0 {
        return base;
    }

    let hydro_albedo = match hydrosphere_state {
        MatterState::Solid => ice_albedo.clamp(0.0, 1.0),
        MatterState::Liquid => liquid_albedo.clamp(0.0, 1.0),
        MatterState::Vapor | MatterState::Supercritical => base,
    };

    (1.0 - cov) * base + cov * hydro_albedo
}
