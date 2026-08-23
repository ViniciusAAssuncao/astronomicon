use crate::climate::resolve_global_mean_temperature;
use crate::error::AppResult;
use crate::geology::resolve_planetary_geology;
use crate::geophysics::resolve_planetary_core;
use crate::hydrosphere::resolve_hydrosphere_diagnostics;
use crate::mineralogy::resolve_planetary_mineralogy;
use astronomicon_core::chemistry::element_mass_fraction;
use astronomicon_core::domain::{ Planet, PlanetKind, PlanetRheology };
use astronomicon_core::error::DomainError;
use astronomicon_core::math::geology::lithosphere_yield_strength;
use astronomicon_core::math::gravity::{ gravitational_parameter, surface_gravity };
use astronomicon_core::math::volcanism::{
    classify_eruption_style,
    cryovolcanic_melt_fraction,
    decompression_melting_temperature,
    depressed_solidus_temperature,
    exsolved_volatile_fraction,
    global_magma_extrusion_rate,
    henry_solubility_h2o,
    magma_density,
    magma_dynamic_viscosity,
    magma_temperature,
    partial_melt_fraction,
    volcanic_outgassing_fluxes,
    VolcanicEruptionStyle,
};
use astronomicon_core::units::constants::{
    SILICATE_LATENT_HEAT_OF_FUSION,
    SILICATE_MELT_SPECIFIC_HEAT,
    WATER_ICE_LATENT_HEAT_OF_FUSION,
};
use astronomicon_core::units::{ Duration, DynamicViscosity, MassRate, Pressure, Temperature };
use astronomicon_db::repositories::{
    atmosphere_repository,
    lithosphere_repository,
    planet_repository,
};
use astronomicon_db::SqlitePool;
use serde::{ Deserialize, Serialize };
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VolcanicDiagnostic {
    pub global_magma_production_rate: MassRate,
    pub magma_temperature: Temperature,
    pub magma_viscosity: DynamicViscosity,
    pub effusive_fraction: f64,
    pub explosive_fraction: f64,
    pub is_magma_ocean: bool,
    pub is_cryovolcanic: bool,
    pub outgassing_rate_h2o: MassRate,
    pub outgassing_rate_co2: MassRate,
    pub outgassing_rate_sulfur: MassRate,
}

pub async fn resolve_planetary_volcanism(
    pool: &SqlitePool,
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration
) -> AppResult<VolcanicDiagnostic> {
    let planet_row = planet_repository
        ::get_by_id(pool, &planet_id).await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let radius = planet.equatorial_radius().ok_or_else(|| DomainError::InvalidInvariant {
        field: "equatorial_radius".to_string(),
        reason: format!("planet '{}' has no equatorial radius", planet_id),
    })?;

    let mu_planet = gravitational_parameter(planet.mass());
    let g = surface_gravity(mu_planet, radius);

    let rheology = match lithosphere_repository::get_by_planet_id(pool, &planet_id).await? {
        Some(r) => r,
        None => PlanetRheology::fallback_for_kind(planet.kind()),
    };

    let surf_temp = resolve_global_mean_temperature(
        pool,
        planet_id,
        universe_epoch,
        at_epoch
    ).await?;
    let core_diag = resolve_planetary_core(pool, planet_id, universe_epoch, at_epoch).await?;
    let geology_diag = resolve_planetary_geology(pool, planet_id, universe_epoch, at_epoch).await?;
    let hydro_diag = resolve_hydrosphere_diagnostics(
        pool,
        planet_id,
        universe_epoch,
        at_epoch
    ).await?;
    let mineralogy_diag = resolve_planetary_mineralogy(
        pool,
        planet_id,
        universe_epoch,
        at_epoch
    ).await?;

    let atm_opt = atmosphere_repository::get_by_planet_id(pool, &planet_id).await?;
    let surface_pressure = atm_opt
        .as_ref()
        .map(|a| a.surface_pressure())
        .unwrap_or(Pressure::new(0.0));

    let is_cryo = matches!(
        planet.kind(),
        PlanetKind::IcyBody | PlanetKind::IceGiant | PlanetKind::DwarfPlanet
    );

    let mantle_hydration = planet.mantle_hydration_fraction().unwrap_or(0.0);
    let dry_solidus = rheology.mean_solidus_temperature();
    let wet_solidus = depressed_solidus_temperature(dry_solidus, mantle_hydration);
    let wet_liquidus = rheology.mean_liquidus_temperature();

    let (
        melt_fraction,
        _extraction_temp,
        magma_temp,
        magma_viscosity_pa_s,
        latent_heat,
        specific_heat,
    ) = if is_cryo {
        let solvent_melting_point = hydro_diag
            .map(|h| h.surface_freezing_point)
            .unwrap_or(dry_solidus);
        let ice_thickness = hydro_diag
            .map(|h| h.ice_thickness)
            .unwrap_or(geology_diag.lithosphere_thickness);
        let solute_fraction = hydro_diag.map(|_| mantle_hydration).unwrap_or(0.0);

        let (frac, t_ext) = cryovolcanic_melt_fraction(
            surf_temp,
            solvent_melting_point,
            core_diag.total_surface_heat_flux,
            solute_fraction,
            ice_thickness,
            rheology.mean_thermal_conductivity()
        );

        let visc = 1.0e-3 * (1.0 + 10.0 * (1.0 - frac));
        (
            frac,
            t_ext,
            t_ext,
            visc,
            WATER_ICE_LATENT_HEAT_OF_FUSION,
            rheology.mean_specific_heat_capacity(),
        )
    } else {
        let t_ext = decompression_melting_temperature(
            wet_solidus,
            g,
            geology_diag.lithosphere_thickness,
            rheology.mean_specific_heat_capacity(),
            rheology.mean_thermal_expansion()
        );
        let frac = partial_melt_fraction(t_ext, wet_solidus, wet_liquidus);
        let t_magma = magma_temperature(t_ext, wet_solidus, wet_liquidus, frac);

        let felsic_frac = mineralogy_diag.crustal_mineralogy.felsic_fraction;
        let silica_fraction = 0.45 + 0.3 * felsic_frac;
        let visc = magma_dynamic_viscosity(t_magma, silica_fraction, mantle_hydration);

        (frac, t_ext, t_magma, visc, SILICATE_LATENT_HEAT_OF_FUSION, SILICATE_MELT_SPECIFIC_HEAT)
    };

    let crust_density = rheology.mean_density();
    let magma_dens = magma_density(
        crust_density,
        melt_fraction,
        magma_temp,
        wet_solidus,
        rheology.mean_thermal_expansion()
    );

    let has_water_weakening =
        hydro_diag.map(|h| h.liquid_depth.value() > 0.0).unwrap_or(false) ||
        mantle_hydration > 0.001;

    let yield_strength = lithosphere_yield_strength(
        rheology.mean_base_yield_stress(),
        has_water_weakening
    );

    let magma_production_rate = global_magma_extrusion_rate(
        geology_diag.tectonic_regime,
        planet.kind(),
        core_diag.total_surface_heat_flux,
        core_diag.cmb_heat_flux,
        radius,
        core_diag.core_radius,
        geology_diag.lithosphere_thickness,
        geology_diag.plate_velocity,
        geology_diag.plate_count,
        crust_density,
        crust_density,
        magma_dens,
        g,
        yield_strength,
        melt_fraction,
        mantle_hydration,
        latent_heat,
        specific_heat,
        magma_temp,
        surf_temp
    );

    let is_magma_ocean =
        surf_temp.value() >= wet_solidus.value() ||
        core_diag.total_surface_heat_flux.value() > 50.0 ||
        melt_fraction >= 0.95;

    let c_o_ratio = mineralogy_diag.abundance.c_o_ratio;
    let sulfur_mass_fraction = element_mass_fraction(
        &mineralogy_diag.abundance.crustal_abundances,
        "S"
    );

    let outgassing_rates = volcanic_outgassing_fluxes(
        magma_production_rate,
        mantle_hydration,
        c_o_ratio,
        sulfur_mass_fraction,
        surface_pressure
    );

    let outgassing_rate_sulfur = outgassing_rates.so2 + outgassing_rates.h2s;

    let is_subaqueous = hydro_diag
        .map(|h| h.liquid_depth.value() > 0.0 && !h.is_completely_frozen)
        .unwrap_or(false);

    let sol_h2o = henry_solubility_h2o(surface_pressure);
    let exsolved_gas = exsolved_volatile_fraction(mantle_hydration, sol_h2o);

    let eruption_style = classify_eruption_style(
        magma_viscosity_pa_s,
        surface_pressure,
        g,
        exsolved_gas,
        is_subaqueous,
        planet.kind(),
        magma_production_rate
    );

    let (effusive_fraction, explosive_fraction) = match eruption_style {
        VolcanicEruptionStyle::Explosive => {
            let exp_frac = (exsolved_gas * 20.0).clamp(0.5, 0.95);
            (1.0 - exp_frac, exp_frac)
        }
        VolcanicEruptionStyle::Effusive | VolcanicEruptionStyle::SubaqueousEffusive => (0.95, 0.05),
        VolcanicEruptionStyle::Cryovolcanic => (0.8, 0.2),
        VolcanicEruptionStyle::Inactive => (0.0, 0.0),
    };

    Ok(VolcanicDiagnostic {
        global_magma_production_rate: magma_production_rate,
        magma_temperature: magma_temp,
        magma_viscosity: DynamicViscosity::new(magma_viscosity_pa_s),
        effusive_fraction,
        explosive_fraction,
        is_magma_ocean,
        is_cryovolcanic: is_cryo,
        outgassing_rate_h2o: outgassing_rates.h2o,
        outgassing_rate_co2: outgassing_rates.co2,
        outgassing_rate_sulfur,
    })
}
