use crate::domain::{ PlanetKind, TectonicRegime };
use crate::math::hydrosphere::HydrosphereStructure;
use crate::units::constants::{
    BASE_LITHOSPHERE_YIELD_STRESS,
    CARBON_CRUST_THERMAL_CONDUCTIVITY,
    CARBON_SOLIDUS_BASE_K,
    EARTH_MASS,
    HEAT_PIPE_HEAT_FLUX_THRESHOLD,
    ICE_CRUST_THERMAL_CONDUCTIVITY,
    ICE_SOLIDUS_BASE_K,
    MANTLE_CONVECTIVE_STRESS_COEFFICIENT,
    MANTLE_DENSITY_REFERENCE,
    MANTLE_THERMAL_EXPANSION,
    PLATE_VELOCITY_SCALING_COEFFICIENT,
    SILICATE_CRUST_THERMAL_CONDUCTIVITY,
    SILICATE_SOLIDUS_BASE_K,
    TECTONIC_PLATE_COUNT_COEFFICIENT,
    WATER_YIELD_STRESS_REDUCTION_FACTOR,
};
use crate::units::{ Acceleration, Density, HeatFlux, Length, Mass, Pressure, Speed, Temperature };

pub fn crust_thermal_conductivity(kind: PlanetKind) -> f64 {
    match kind {
        PlanetKind::Telluric | PlanetKind::Chthonian => SILICATE_CRUST_THERMAL_CONDUCTIVITY,
        PlanetKind::CarbonPlanet => CARBON_CRUST_THERMAL_CONDUCTIVITY,
        PlanetKind::IcyBody | PlanetKind::IceGiant | PlanetKind::DwarfPlanet => {
            ICE_CRUST_THERMAL_CONDUCTIVITY
        }
        PlanetKind::GasGiant | PlanetKind::Exotic => SILICATE_CRUST_THERMAL_CONDUCTIVITY,
    }
}

pub fn mantle_solidus_temperature(kind: PlanetKind, mass: Mass) -> Temperature {
    let m = mass.value();
    if m <= 0.0 || !m.is_finite() {
        return Temperature::new(0.0);
    }

    let base_solidus = match kind {
        PlanetKind::Telluric | PlanetKind::Chthonian => SILICATE_SOLIDUS_BASE_K,
        PlanetKind::CarbonPlanet => CARBON_SOLIDUS_BASE_K,
        PlanetKind::IcyBody | PlanetKind::IceGiant | PlanetKind::DwarfPlanet => ICE_SOLIDUS_BASE_K,
        PlanetKind::GasGiant | PlanetKind::Exotic => SILICATE_SOLIDUS_BASE_K,
    };

    let mass_ratio = m / EARTH_MASS;
    let scaling = (1.0 + 0.05 * (1.0 + mass_ratio).ln().max(0.0)).max(0.1);
    let t_solidus = base_solidus * scaling;

    if !t_solidus.is_finite() || t_solidus <= 0.0 {
        Temperature::new(0.0)
    } else {
        Temperature::new(t_solidus)
    }
}

pub fn lithosphere_thickness(
    base_temperature: Temperature,
    surface_temperature: Temperature,
    surface_heat_flux: HeatFlux,
    thermal_conductivity: f64
) -> Length {
    let t_base = base_temperature.value();
    let t_surf = surface_temperature.value();
    let q = surface_heat_flux.value();
    let k = thermal_conductivity;

    if
        !t_base.is_finite() ||
        !t_surf.is_finite() ||
        !q.is_finite() ||
        !k.is_finite() ||
        k <= 0.0 ||
        t_base <= t_surf
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

pub fn lithosphere_thickness_for_planet(
    kind: PlanetKind,
    mass: Mass,
    surface_temperature: Temperature,
    surface_heat_flux: HeatFlux
) -> Length {
    let t_solidus = mantle_solidus_temperature(kind, mass);
    let k = crust_thermal_conductivity(kind);
    lithosphere_thickness(t_solidus, surface_temperature, surface_heat_flux, k)
}

pub fn mantle_convective_stress(
    surface_gravity: Acceleration,
    surface_heat_flux: HeatFlux,
    mantle_density: Density,
    thermal_expansion: f64
) -> Pressure {
    let g = surface_gravity.value();
    let q = surface_heat_flux.value();
    let rho = mantle_density.value();
    let alpha = thermal_expansion;

    if
        g <= 0.0 ||
        q <= 0.0 ||
        rho <= 0.0 ||
        alpha <= 0.0 ||
        !g.is_finite() ||
        !q.is_finite() ||
        !rho.is_finite() ||
        !alpha.is_finite()
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
    has_water_weakening: bool
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
    yield_strength: Pressure
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
    hydrosphere_structure: Option<&HydrosphereStructure>
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

    if
        r_p <= 0.0 ||
        z_l >= r_p ||
        q_surf <= 0.0 ||
        g <= 0.0 ||
        !r_p.is_finite() ||
        !z_l.is_finite() ||
        !q_surf.is_finite() ||
        !g.is_finite()
    {
        return TectonicRegime::Inactive;
    }

    let mantle_density = Density::new(MANTLE_DENSITY_REFERENCE);
    let conv_stress = mantle_convective_stress(
        surface_gravity,
        surface_heat_flux,
        mantle_density,
        MANTLE_THERMAL_EXPANSION
    );

    let yield_strength = lithosphere_yield_strength(
        Pressure::new(BASE_LITHOSPHERE_YIELD_STRESS),
        has_water
    );

    if conv_stress.value() >= yield_strength.value() {
        TectonicRegime::PlateTectonics
    } else {
        TectonicRegime::StagnantLid
    }
}

pub fn tectonic_plate_count(
    planet_radius: Length,
    lithosphere_thickness: Length,
    regime: TectonicRegime
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
