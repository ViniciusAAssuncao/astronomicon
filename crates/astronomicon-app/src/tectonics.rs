use crate::geophysics::PlanetaryCoreDiagnostic;
use astronomicon_core::domain::{Planet, PlanetRheology, TectonicRegime};
use astronomicon_core::math::geology::{
    brittle_ductile_transition_depth, determine_tectonic_regime, lithosphere_thickness,
    lithosphere_yield_strength, plate_rms_velocity, tectonic_plate_count,
};
use astronomicon_core::math::hydrosphere::HydrosphereStructure;
use astronomicon_core::math::seismology::seismic_efficiency;
use astronomicon_core::units::{Acceleration, Length, Pressure, Speed, Temperature};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TectonicSetup {
    pub regime: TectonicRegime,
    pub z_lith: Length,
    pub z_brittle: Length,
    pub plate_count: u32,
    pub plate_velocity: Speed,
    pub yield_strength: Pressure,
    pub seismic_efficiency: f64,
}

pub fn resolve_tectonic_setup(
    planet: &Planet,
    rheology: &PlanetRheology,
    radius: Length,
    gravity: Acceleration,
    surface_temp: Temperature,
    core_diag: &PlanetaryCoreDiagnostic,
    has_water_weakening: bool,
    hydro_structure: Option<&HydrosphereStructure>,
) -> TectonicSetup {
    let z_lith = lithosphere_thickness(
        rheology.mean_solidus_temperature(),
        surface_temp,
        core_diag.total_surface_heat_flux,
        rheology.mean_thermal_conductivity(),
    );

    let z_brittle = brittle_ductile_transition_depth(
        z_lith,
        surface_temp,
        rheology.mean_solidus_temperature(),
        rheology.mean_solidus_temperature(),
    );

    let regime = determine_tectonic_regime(
        planet.kind(),
        radius,
        z_lith,
        gravity,
        core_diag.total_surface_heat_flux,
        core_diag.tidal_heat_flux,
        has_water_weakening,
        hydro_structure,
        rheology,
    );

    let plate_count = tectonic_plate_count(radius, z_lith, regime);
    let plate_velocity = plate_rms_velocity(core_diag.convective_heat_flux, regime);

    let yield_strength =
        lithosphere_yield_strength(rheology.mean_base_yield_stress(), has_water_weakening);

    let seismic_eff = seismic_efficiency(yield_strength, rheology.mean_shear_modulus());

    TectonicSetup {
        regime,
        z_lith,
        z_brittle,
        plate_count,
        plate_velocity,
        yield_strength,
        seismic_efficiency: seismic_eff,
    }
}
