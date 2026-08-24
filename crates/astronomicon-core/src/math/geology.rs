use crate::domain::{PlanetKind, PlanetRheology, TectonicRegime};
use crate::math::hydrosphere::HydrosphereStructure;
use crate::units::constants::{
    HEAT_PIPE_HEAT_FLUX_THRESHOLD, MANTLE_CONVECTIVE_STRESS_COEFFICIENT,
    PLATE_VELOCITY_SCALING_COEFFICIENT, TECTONIC_PLATE_COUNT_COEFFICIENT,
    WATER_YIELD_STRESS_REDUCTION_FACTOR,
};
use crate::units::{Acceleration, Density, HeatFlux, Length, Pressure, Speed, Temperature};

pub fn lithosphere_thickness(
    base_temperature: Temperature,
    surface_temperature: Temperature,
    surface_heat_flux: HeatFlux,
    thermal_conductivity: f64,
) -> Length {
    let t_base = base_temperature.value();
    let t_surf = surface_temperature.value();
    let q = surface_heat_flux.value();
    let k = thermal_conductivity;

    if !t_base.is_finite()
        || !t_surf.is_finite()
        || !q.is_finite()
        || !k.is_finite()
        || k <= 0.0
        || t_base <= t_surf
    {
        return Length::new(0.0);
    }

    if q <= 0.0 {
        return Length::new(f64::INFINITY);
    }

    let delta_t = t_base - t_surf;
    let z_l = k * (delta_t / q);

    if !z_l.is_finite() || z_l <= 0.0 {
        Length::new(0.0)
    } else {
        Length::new(z_l)
    }
}

pub fn brittle_ductile_transition_depth(
    lithosphere_thickness: Length,
    surface_temperature: Temperature,
    base_temperature: Temperature,
    solidus_temperature: Temperature,
) -> Length {
    let z_l = lithosphere_thickness.value();
    let t_surf = surface_temperature.value();
    let t_base = base_temperature.value();
    let t_sol = solidus_temperature.value();

    if !z_l.is_finite()
        || z_l <= 0.0
        || !t_surf.is_finite()
        || !t_base.is_finite()
        || !t_sol.is_finite()
        || t_base <= t_surf
    {
        return Length::new(0.0);
    }

    let delta_t = t_base - t_surf;
    let delta_t_brittle = 0.5 * t_sol - t_surf;

    if delta_t_brittle <= 0.0 {
        return Length::new(0.0);
    }

    let fraction = (delta_t_brittle / delta_t).clamp(0.0, 1.0);
    Length::new(z_l * fraction)
}

pub fn mantle_convective_stress(
    surface_gravity: Acceleration,
    surface_heat_flux: HeatFlux,
    mantle_density: Density,
    thermal_expansion: f64,
) -> Pressure {
    let g = surface_gravity.value();
    let q = surface_heat_flux.value();
    let rho = mantle_density.value();
    let alpha = thermal_expansion;

    if g <= 0.0
        || q <= 0.0
        || rho <= 0.0
        || alpha <= 0.0
        || !g.is_finite()
        || !q.is_finite()
        || !rho.is_finite()
        || !alpha.is_finite()
    {
        return Pressure::new(0.0);
    }

    let stress = (rho * g * alpha * q).powf(2.0 / 3.0) * MANTLE_CONVECTIVE_STRESS_COEFFICIENT;
    if !stress.is_finite() || stress <= 0.0 {
        Pressure::new(0.0)
    } else {
        Pressure::new(stress)
    }
}

pub fn lithosphere_yield_strength(
    base_yield_stress: Pressure,
    has_water_weakening: bool,
) -> Pressure {
    let base = base_yield_stress.value();
    if base <= 0.0 || !base.is_finite() {
        return Pressure::new(0.0);
    }

    if has_water_weakening {
        Pressure::new(base * WATER_YIELD_STRESS_REDUCTION_FACTOR)
    } else {
        base_yield_stress
    }
}

pub fn convective_to_yield_stress_ratio(
    convective_stress: Pressure,
    yield_strength: Pressure,
) -> f64 {
    let tau = convective_stress.value();
    let sigma_y = yield_strength.value();

    if tau <= 0.0 || sigma_y <= 0.0 || !tau.is_finite() || !sigma_y.is_finite() {
        0.0
    } else {
        tau / sigma_y
    }
}

pub fn determine_tectonic_regime(
    kind: PlanetKind,
    planet_radius: Length,
    lithosphere_thickness: Length,
    surface_gravity: Acceleration,
    surface_heat_flux: HeatFlux,
    tidal_heat_flux: HeatFlux,
    has_water: bool,
    hydrosphere_structure: Option<&HydrosphereStructure>,
    rheology: &PlanetRheology,
) -> TectonicRegime {
    if surface_heat_flux.value() >= HEAT_PIPE_HEAT_FLUX_THRESHOLD {
        return TectonicRegime::HeatPipe;
    }

    let is_ice_crust = matches!(
        kind,
        PlanetKind::IcyBody | PlanetKind::IceGiant | PlanetKind::DwarfPlanet
    );

    if is_ice_crust {
        if let Some(hydro) = hydrosphere_structure {
            if hydro.is_subsurface_ocean && tidal_heat_flux.value() > 0.0 {
                return TectonicRegime::IceTectonics;
            }
        }
    }

    let r_p = planet_radius.value();
    let z_l = lithosphere_thickness.value();
    let q_surf = surface_heat_flux.value();
    let g = surface_gravity.value();

    if r_p <= 0.0
        || z_l >= r_p
        || q_surf <= 0.0
        || g <= 0.0
        || !r_p.is_finite()
        || !z_l.is_finite()
        || !q_surf.is_finite()
        || !g.is_finite()
    {
        return TectonicRegime::Inactive;
    }

    let mantle_density = rheology.mean_density();
    let thermal_expansion = rheology.mean_thermal_expansion();
    let conv_stress = mantle_convective_stress(
        surface_gravity,
        surface_heat_flux,
        mantle_density,
        thermal_expansion,
    );

    let base_yield = rheology.mean_base_yield_stress();
    let yield_strength = lithosphere_yield_strength(base_yield, has_water);

    if conv_stress.value() >= yield_strength.value() {
        TectonicRegime::PlateTectonics
    } else {
        TectonicRegime::StagnantLid
    }
}

pub fn tectonic_plate_count(
    planet_radius: Length,
    lithosphere_thickness: Length,
    regime: TectonicRegime,
) -> u32 {
    match regime {
        TectonicRegime::PlateTectonics | TectonicRegime::IceTectonics => {
            let r = planet_radius.value();
            let z = lithosphere_thickness.value();
            if r <= 0.0 || z <= 0.0 || !r.is_finite() || !z.is_finite() {
                return 2;
            }
            let ratio = r / z;
            let count = (TECTONIC_PLATE_COUNT_COEFFICIENT * ratio * ratio).round() as u32;
            count.max(2)
        }
        TectonicRegime::StagnantLid | TectonicRegime::HeatPipe => 1,
        TectonicRegime::Inactive => 0,
    }
}

pub fn plate_rms_velocity(convective_heat_flux: HeatFlux, regime: TectonicRegime) -> Speed {
    match regime {
        TectonicRegime::PlateTectonics | TectonicRegime::IceTectonics => {
            let q = convective_heat_flux.value();
            if q <= 0.0 || !q.is_finite() {
                return Speed::new(0.0);
            }
            let v = PLATE_VELOCITY_SCALING_COEFFICIENT * q * q;
            if !v.is_finite() || v <= 0.0 {
                Speed::new(0.0)
            } else {
                Speed::new(v)
            }
        }
        TectonicRegime::StagnantLid | TectonicRegime::HeatPipe | TectonicRegime::Inactive => {
            Speed::new(0.0)
        }
    }
}
