use crate::domain::TectonicRegime;
use crate::units::{Duration, HeatFlux, Temperature};

pub fn hydrothermal_vein_potential(
    bulk_abundance: f64,
    has_water: bool,
    is_liquid_or_supercritical: bool,
    convective_heat_flux: HeatFlux,
    regime: TectonicRegime,
) -> (f64, f64) {
    if !has_water || !is_liquid_or_supercritical || bulk_abundance <= 0.0 {
        return (0.0, 1.0);
    }

    let q = convective_heat_flux.value();
    if q <= 0.0 || !q.is_finite() {
        return (0.0, 1.0);
    }

    let regime_mult = match regime {
        TectonicRegime::PlateTectonics => 1.0,
        TectonicRegime::HeatPipe => 0.85,
        TectonicRegime::IceTectonics => 0.40,
        TectonicRegime::StagnantLid => 0.30,
        TectonicRegime::Inactive => 0.0,
    };

    if regime_mult <= 0.0 {
        return (0.0, 1.0);
    }

    let f_q = (q / 0.05).clamp(0.0, 3.0);
    let prob = (0.25 * regime_mult * (1.0 + f_q)).clamp(0.0, 0.98);
    let enrichment = 50.0 + 450.0 * (1.0 - (-f_q * regime_mult).exp());

    (prob, enrichment)
}

pub fn evaporite_deposit_potential(
    bulk_abundance: f64,
    has_water: bool,
    surface_temp: Temperature,
    boiling_point: Temperature,
    salinity: f64,
    ocean_coverage: f64,
) -> (f64, f64) {
    if !has_water || bulk_abundance <= 0.0 || salinity <= 0.0 {
        return (0.0, 1.0);
    }

    let t_surf = surface_temp.value();
    let t_boil = boiling_point.value();

    if t_surf < 260.0 || t_boil <= 0.0 || !t_surf.is_finite() || !t_boil.is_finite() {
        return (0.0, 1.0);
    }

    let f_t = (t_surf / t_boil).clamp(0.0, 1.0);
    let f_evap =
        ((f_t - 0.65) / 0.35).clamp(0.0, 1.0) * 0.6 + 0.4 * (t_surf / 373.15).clamp(0.0, 1.0);
    let f_basin = (4.0 * ocean_coverage * (1.0 - ocean_coverage)).clamp(0.1, 1.0);
    let f_sal = (salinity / 0.035).clamp(0.1, 3.0);

    let prob = (0.35 * f_evap * f_basin * (f_sal / 2.0)).clamp(0.0, 0.95);
    let enrichment = 20.0 + 180.0 * (f_sal * f_evap).clamp(0.0, 5.0);

    (prob, enrichment)
}

pub fn banded_iron_formation_potential(
    fe_crustal_abundance: f64,
    has_water: bool,
    is_liquid_ocean: bool,
    has_oxidizing_atmosphere: bool,
    age: Duration,
) -> (f64, f64) {
    if !has_water || !is_liquid_ocean || fe_crustal_abundance <= 0.0 || !has_oxidizing_atmosphere {
        return (0.0, 1.0);
    }

    let t_yr = age.value() / 31557600.0;
    if !t_yr.is_finite() || t_yr <= 0.0 {
        return (0.0, 1.0);
    }

    let f_age = if t_yr < 5.0e8 {
        (t_yr / 5.0e8).clamp(0.0, 1.0)
    } else {
        1.0
    };

    let f_fe = (fe_crustal_abundance / 0.05).clamp(0.1, 2.5);
    let prob = (0.75 * f_age * f_fe).clamp(0.0, 0.95);
    let enrichment = 5.0 + 15.0 * f_fe;

    (prob, enrichment)
}

pub fn magmatic_sulfide_potential(
    ni_abundance: f64,
    cu_abundance: f64,
    core_mass_fraction: f64,
    convective_heat_flux: HeatFlux,
    regime: TectonicRegime,
) -> (f64, f64) {
    if regime == TectonicRegime::Inactive || convective_heat_flux.value() <= 0.0 {
        return (0.0, 1.0);
    }

    let regime_mult = match regime {
        TectonicRegime::PlateTectonics => 1.0,
        TectonicRegime::HeatPipe => 1.2,
        TectonicRegime::StagnantLid => 0.4,
        TectonicRegime::IceTectonics => 0.1,
        TectonicRegime::Inactive => 0.0,
    };

    let metal_factor = ((ni_abundance + cu_abundance) / 0.002).clamp(0.1, 3.0);
    let core_factor = (core_mass_fraction / 0.325).clamp(0.2, 1.5);

    let prob = (0.35 * regime_mult * metal_factor * core_factor).clamp(0.0, 0.95);
    let enrichment = 15.0 + 85.0 * metal_factor;

    (prob, enrichment)
}

pub fn pegmatite_ree_potential(
    felsic_fraction: f64,
    regime: TectonicRegime,
    age: Duration,
) -> (f64, f64) {
    if felsic_fraction < 0.15 {
        return (0.0, 1.0);
    }

    let age_factor = (age.value() / (2.0e9 * 31557600.0)).clamp(0.0, 1.5);
    let regime_mult = match regime {
        TectonicRegime::PlateTectonics => 1.0,
        TectonicRegime::StagnantLid => 0.3,
        TectonicRegime::HeatPipe => 0.2,
        TectonicRegime::IceTectonics => 0.1,
        TectonicRegime::Inactive => 0.0,
    };

    let prob = (0.50 * felsic_fraction * regime_mult * age_factor).clamp(0.0, 0.95);
    let enrichment = 30.0 + 120.0 * felsic_fraction;

    (prob, enrichment)
}
