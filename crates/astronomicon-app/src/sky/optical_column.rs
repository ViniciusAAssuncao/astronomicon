use crate::climate::circulation::resolve_wind_profile_at_latitude;
use crate::climate::clouds::cover::{
    CloudCoverDiagnostic,
    resolve_cloud_cover,
    resolve_cloud_cover_at_latitude,
};
use crate::climate::clouds::instability::{
    ConvectiveInstabilityDiagnostic,
    resolve_convective_instability,
    resolve_convective_instability_at_latitude,
};
use crate::climate::clouds::layer::CloudLayerDiagnostic;
use crate::climate::clouds::tropopause::{ resolve_tropopause, resolve_tropopause_at_latitude };
use crate::climate::condensable_species::resolve_condensable_species;
use crate::climate::temperature::{
    resolve_advective_surface_temperature,
    resolve_global_mean_temperature,
};
use crate::error::AppResult;
use crate::mineralogy::resolve_planetary_mineralogy;
use crate::volcanism::{ VolcanicDiagnostic, resolve_planetary_volcanism };
use astronomicon_core::chemistry::element_mass_fraction;
use astronomicon_core::chemistry::optics::mean_gas_optical_properties;
use astronomicon_core::chemistry::solvent::SolventProperties;
use astronomicon_core::domain::{ Atmosphere, Planet };
use astronomicon_core::error::DomainError;
use astronomicon_core::math::aerosol::{
    airborne_dust_density_with_gustiness,
    dust_threshold_surface_wind_with_params,
    dynamic_aerosol_scale_height,
    volcanic_aerosol_density,
};
use astronomicon_core::math::atmosphere::ideal_gas_density;
use astronomicon_core::math::climate::temperature_at_altitude;
use astronomicon_core::math::clouds::CloudMorphology;
use astronomicon_core::math::gravity::{ gravitational_parameter, surface_gravity };
use astronomicon_core::math::optics::{
    absorption_coefficient,
    particulate_optical_properties,
    rayleigh_scattering_coefficient,
};
use astronomicon_core::math::precipitation::{
    layer_vertical_velocity_scale,
    resolve_sedimentation_balance,
};
use astronomicon_core::math::volcanism::VolcanicEruptionStyle;
use astronomicon_core::units::constants::{
    DEFAULT_DUST_PARTICLE_RADIUS_M,
    DEFAULT_VOLCANIC_ASH_PARTICLE_RADIUS_M,
};
use astronomicon_core::units::{
    Acceleration,
    Angle,
    Density,
    Duration,
    DynamicViscosity,
    Length,
    MolarMass,
    Pressure,
    SpecificEnergy,
    Speed,
    Temperature,
    TemperatureGradient,
    Wavelength,
};
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::{
    atmosphere_repository,
    hydrosphere_repository,
    planet_repository,
};
use serde::{ Deserialize, Serialize };
use uuid::Uuid;

pub const WAVELENGTH_RED_METERS: f64 = 680.0e-9;
pub const WAVELENGTH_GREEN_METERS: f64 = 550.0e-9;
pub const WAVELENGTH_BLUE_METERS: f64 = 440.0e-9;

pub fn wavelength_red() -> Wavelength {
    Wavelength::new(WAVELENGTH_RED_METERS)
}

pub fn wavelength_green() -> Wavelength {
    Wavelength::new(WAVELENGTH_GREEN_METERS)
}

pub fn wavelength_blue() -> Wavelength {
    Wavelength::new(WAVELENGTH_BLUE_METERS)
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SpectralOpticalDepth {
    pub rayleigh_r: f64,
    pub rayleigh_g: f64,
    pub rayleigh_b: f64,
    pub gas_absorption_r: f64,
    pub gas_absorption_g: f64,
    pub gas_absorption_b: f64,
    pub dust_r: f64,
    pub dust_g: f64,
    pub dust_b: f64,
    pub volcanic_r: f64,
    pub volcanic_g: f64,
    pub volcanic_b: f64,
    pub cloud_r: f64,
    pub cloud_g: f64,
    pub cloud_b: f64,
    pub aerosol_r: f64,
    pub aerosol_g: f64,
    pub aerosol_b: f64,
    pub total_r: f64,
    pub total_g: f64,
    pub total_b: f64,
    pub single_scattering_albedo_r: f64,
    pub single_scattering_albedo_g: f64,
    pub single_scattering_albedo_b: f64,
    pub asymmetry_factor_r: f64,
    pub asymmetry_factor_g: f64,
    pub asymmetry_factor_b: f64,
}

impl SpectralOpticalDepth {
    pub fn rayleigh(&self) -> (f64, f64, f64) {
        (self.rayleigh_r, self.rayleigh_g, self.rayleigh_b)
    }

    pub fn gas_absorption(&self) -> (f64, f64, f64) {
        (self.gas_absorption_r, self.gas_absorption_g, self.gas_absorption_b)
    }

    pub fn dust(&self) -> (f64, f64, f64) {
        (self.dust_r, self.dust_g, self.dust_b)
    }

    pub fn volcanic(&self) -> (f64, f64, f64) {
        (self.volcanic_r, self.volcanic_g, self.volcanic_b)
    }

    pub fn cloud(&self) -> (f64, f64, f64) {
        (self.cloud_r, self.cloud_g, self.cloud_b)
    }

    pub fn aerosol(&self) -> (f64, f64, f64) {
        (self.aerosol_r, self.aerosol_g, self.aerosol_b)
    }

    pub fn total(&self) -> (f64, f64, f64) {
        (self.total_r, self.total_g, self.total_b)
    }

    pub fn single_scattering_albedo(&self) -> (f64, f64, f64) {
        (
            self.single_scattering_albedo_r,
            self.single_scattering_albedo_g,
            self.single_scattering_albedo_b,
        )
    }

    pub fn asymmetry_factor(&self) -> (f64, f64, f64) {
        (self.asymmetry_factor_r, self.asymmetry_factor_g, self.asymmetry_factor_b)
    }
}

pub fn compute_cloud_layer_droplet_radius(
    layer: &CloudLayerDiagnostic,
    surf_temp: Temperature,
    lapse_rate: TemperatureGradient,
    scale_h: Length,
    surface_pressure: Pressure,
    atm_molar_mass: MolarMass,
    mean_viscosity: DynamicViscosity,
    solvent_liquid_density: Density,
    solvent_solid_density: Density,
    morphology: CloudMorphology,
    cape: SpecificEnergy,
    vertical_wind_shear: f64,
    gravity: Acceleration,
    ccn_factor: Option<f64>
) -> Length {
    let dz = Length::new((layer.top_altitude.value() - layer.base_altitude.value()).max(0.0));
    let rho_cond = Density::new(
        layer.liquid_water_content.value() + layer.ice_water_content.value()
    );
    let ice_frac = layer.ice_fraction.clamp(0.0, 1.0);
    let rho_p_val = (
        (1.0 - ice_frac) * solvent_liquid_density.value() +
        ice_frac * solvent_solid_density.value()
    ).max(1.0);
    let particle_density = Density::new(rho_p_val);

    let temp_z = temperature_at_altitude(surf_temp, layer.representative_altitude, lapse_rate);
    let exponent = -layer.representative_altitude.value() / scale_h.value().max(1.0);
    let press_z = Pressure::new(surface_pressure.value() * exponent.exp());
    let fluid_density = ideal_gas_density(press_z, atm_molar_mass, temp_z);

    let w_scale = layer_vertical_velocity_scale(morphology, cape, dz, vertical_wind_shear);

    let sed_res = resolve_sedimentation_balance(
        rho_cond,
        particle_density,
        fluid_density,
        mean_viscosity,
        gravity,
        w_scale,
        ccn_factor
    );

    if sed_res.sedimentable_fraction > 0.05 {
        Length::new(sed_res.critical_radius.value().max(sed_res.mean_droplet_radius.value()))
    } else if sed_res.mean_droplet_radius.value() > 0.0 {
        sed_res.mean_droplet_radius
    } else {
        Length::new(10.0e-6)
    }
}

pub fn calculate_spectral_optical_depth(
    atmosphere: &Atmosphere,
    planet: &Planet,
    surface_temperature: Temperature,
    surface_wind_speed: Speed,
    volc_diag: &VolcanicDiagnostic,
    ocean_coverage: f64,
    surface_humidity: f64,
    scale_height: Length,
    gravity: Acceleration,
    cloud_diag: &CloudCoverDiagnostic,
    instability: &ConvectiveInstabilityDiagnostic,
    vertical_wind_shear: f64,
    solvent_props: &SolventProperties,
    crustal_iron_fraction: Option<f64>
) -> AppResult<SpectralOpticalDepth> {
    let atm_composition: Vec<(String, f64)> = atmosphere
        .composition()
        .iter()
        .map(|c| (c.formula().to_string(), c.percentage()))
        .collect();

    let gas_opt_props = mean_gas_optical_properties(&atm_composition)?;
    let atm_molar_mass = atmosphere.mean_molar_mass()?;
    let surf_press = atmosphere.surface_pressure();
    let dyn_visc = atmosphere
        .mean_dynamic_viscosity(surface_temperature)
        .unwrap_or_else(|_| DynamicViscosity::new(1.81e-5));
    let rho_air_surf = ideal_gas_density(surf_press, atm_molar_mass, surface_temperature);
    let scale_h_val = scale_height.value().max(1.0);

    let wavelengths = [wavelength_red(), wavelength_green(), wavelength_blue()];

    let mut tau_rayleigh = [0.0; 3];
    let mut tau_gas_abs = [0.0; 3];

    for (i, &lambda) in wavelengths.iter().enumerate() {
        let beta_r = rayleigh_scattering_coefficient(
            lambda,
            gas_opt_props.refractivity_stp(),
            gas_opt_props.king_factor(),
            surf_press,
            surface_temperature
        );
        tau_rayleigh[i] = (beta_r * scale_h_val).max(0.0);

        let beta_abs = absorption_coefficient(
            &gas_opt_props,
            lambda,
            surf_press,
            surface_temperature
        );
        tau_gas_abs[i] = (beta_abs * scale_h_val).max(0.0);
    }

    let grain_density = Density::new(2650.0);
    let dust_avail = planet
        .dust_availability_factor()
        .unwrap_or((1.0 - ocean_coverage).clamp(0.0, 1.0));
    let v_thresh = dust_threshold_surface_wind_with_params(
        gravity,
        rho_air_surf,
        grain_density,
        dyn_visc,
        Some(0.003)
    );
    let rho_dust_surf = airborne_dust_density_with_gustiness(
        surface_wind_speed,
        v_thresh,
        rho_air_surf,
        gravity,
        dust_avail,
        ocean_coverage,
        surface_humidity,
        2.0
    );
    let r_dust = planet
        .dust_particle_radius()
        .unwrap_or_else(|| Length::new(DEFAULT_DUST_PARTICLE_RADIUS_M));
    let h_dust = dynamic_aerosol_scale_height(
        scale_height,
        gravity,
        1.2,
        grain_density,
        rho_air_surf,
        r_dust,
        dyn_visc
    );
    let h_dust_val = if h_dust.value() > 0.0 { h_dust.value() } else { 0.2 * scale_h_val };
    let m_col_dust = rho_dust_surf.value() * h_dust_val;

    let n_r_dust = 1.53;
    let fe_frac = crustal_iron_fraction.unwrap_or(0.05).clamp(0.0, 1.0);

    let mut tau_ext_dust = [0.0; 3];
    let mut tau_sca_dust = [0.0; 3];
    let mut g_dust = [0.0; 3];

    for (i, &lambda) in wavelengths.iter().enumerate() {
        let spectral_scale = (550.0e-9 / lambda.value()).powi(3);
        let n_i_dust = (0.001 + 0.04 * fe_frac * spectral_scale).max(1e-5);
        let opt_dust = particulate_optical_properties(
            r_dust,
            grain_density,
            n_r_dust,
            n_i_dust,
            lambda
        );
        let ext = (m_col_dust * opt_dust.mass_extinction_coefficient()).max(0.0);
        tau_ext_dust[i] = ext;
        tau_sca_dust[i] = (ext * opt_dust.single_scattering_albedo()).max(0.0);
        g_dust[i] = opt_dust.asymmetry_factor();
    }

    let planet_radius = planet.equatorial_radius().unwrap_or_else(|| Length::new(6371e3));
    let eruption_style = if volc_diag.is_cryovolcanic {
        VolcanicEruptionStyle::Cryovolcanic
    } else if volc_diag.explosive_fraction > 0.3 {
        VolcanicEruptionStyle::Explosive
    } else if volc_diag.global_magma_production_rate.value() > 0.0 {
        VolcanicEruptionStyle::Effusive
    } else {
        VolcanicEruptionStyle::Inactive
    };
    let rho_volc_surf = volcanic_aerosol_density(
        volc_diag.outgassing_rate_sulfur,
        eruption_style,
        volc_diag.global_magma_production_rate,
        planet_radius,
        scale_height
    );
    let r_volc = planet
        .volcanic_ash_particle_radius()
        .unwrap_or_else(|| Length::new(DEFAULT_VOLCANIC_ASH_PARTICLE_RADIUS_M));
    let rho_p_volc = Density::new(2300.0);
    let h_volc = dynamic_aerosol_scale_height(
        scale_height,
        gravity,
        1.2,
        rho_p_volc,
        rho_air_surf,
        r_volc,
        dyn_visc
    );
    let h_volc_val = if h_volc.value() > 0.0 { h_volc.value() } else { 0.2 * scale_h_val };
    let m_col_volc = rho_volc_surf.value() * h_volc_val;

    let n_r_volc = 1.5;
    let n_i_volc = 0.0015;

    let mut tau_ext_volc = [0.0; 3];
    let mut tau_sca_volc = [0.0; 3];
    let mut g_volc = [0.0; 3];

    for (i, &lambda) in wavelengths.iter().enumerate() {
        let opt_volc = particulate_optical_properties(
            r_volc,
            rho_p_volc,
            n_r_volc,
            n_i_volc,
            lambda
        );
        let ext = (m_col_volc * opt_volc.mass_extinction_coefficient()).max(0.0);
        tau_ext_volc[i] = ext;
        tau_sca_volc[i] = (ext * opt_volc.single_scattering_albedo()).max(0.0);
        g_volc[i] = opt_volc.asymmetry_factor();
    }

    let ccn_factor = atmosphere.cloud_condensation_nuclei_factor();
    let cloud_layers = [&cloud_diag.low_cloud, &cloud_diag.mid_cloud, &cloud_diag.high_cloud];

    let mut tau_ext_cloud = [0.0; 3];
    let mut tau_sca_cloud = [0.0; 3];
    let mut g_cloud_weighted = [0.0; 3];

    for layer in cloud_layers {
        let dz = (layer.top_altitude.value() - layer.base_altitude.value()).max(0.0);
        let rho_cond = layer.liquid_water_content.value() + layer.ice_water_content.value();
        let col_mass = (rho_cond * dz * layer.cloud_fraction).max(0.0);

        if col_mass <= 0.0 {
            continue;
        }

        let r_drop = compute_cloud_layer_droplet_radius(
            layer,
            surface_temperature,
            atmosphere.lapse_rate(),
            scale_height,
            surf_press,
            atm_molar_mass,
            dyn_visc,
            solvent_props.liquid_density,
            solvent_props.solid_density,
            instability.morphology,
            instability.cape,
            vertical_wind_shear,
            gravity,
            ccn_factor
        );

        let ice_f = layer.ice_fraction.clamp(0.0, 1.0);
        let rho_p_layer = Density::new(
            (
                (1.0 - ice_f) * solvent_props.liquid_density.value() +
                ice_f * solvent_props.solid_density.value()
            ).max(1.0)
        );
        let n_r_layer =
            (1.0 - ice_f) * solvent_props.liquid_refractive_index_real +
            ice_f * solvent_props.solid_refractive_index_real;
        let n_i_layer =
            (1.0 - ice_f) * solvent_props.liquid_refractive_index_imag +
            ice_f * solvent_props.solid_refractive_index_imag;

        for (i, &lambda) in wavelengths.iter().enumerate() {
            let opt_layer = particulate_optical_properties(
                r_drop,
                rho_p_layer,
                n_r_layer,
                n_i_layer,
                lambda
            );
            let ext_layer = col_mass * opt_layer.mass_extinction_coefficient();
            let sca_layer = ext_layer * opt_layer.single_scattering_albedo();
            tau_ext_cloud[i] += ext_layer;
            tau_sca_cloud[i] += sca_layer;
            g_cloud_weighted[i] += sca_layer * opt_layer.asymmetry_factor();
        }
    }

    let mut g_cloud = [0.0; 3];
    for i in 0..3 {
        if tau_sca_cloud[i] > 1e-12 {
            g_cloud[i] = g_cloud_weighted[i] / tau_sca_cloud[i];
        }
    }

    let mut tau_aerosol = [0.0; 3];
    let mut tau_total = [0.0; 3];
    let mut tau_sca_total = [0.0; 3];
    let mut ssa_total = [1.0; 3];
    let mut g_total = [0.0; 3];

    for i in 0..3 {
        tau_aerosol[i] = tau_ext_dust[i] + tau_ext_volc[i] + tau_ext_cloud[i];
        tau_total[i] = tau_rayleigh[i] + tau_gas_abs[i] + tau_aerosol[i];
        tau_sca_total[i] = tau_rayleigh[i] + tau_sca_dust[i] + tau_sca_volc[i] + tau_sca_cloud[i];

        if tau_total[i] > 1e-12 {
            ssa_total[i] = (tau_sca_total[i] / tau_total[i]).clamp(0.0, 1.0);
        } else {
            ssa_total[i] = 1.0;
        }

        if tau_sca_total[i] > 1e-12 {
            let num =
                tau_sca_dust[i] * g_dust[i] +
                tau_sca_volc[i] * g_volc[i] +
                tau_sca_cloud[i] * g_cloud[i];
            g_total[i] = (num / tau_sca_total[i]).clamp(-0.999, 0.999);
        } else {
            g_total[i] = 0.0;
        }
    }

    Ok(SpectralOpticalDepth {
        rayleigh_r: tau_rayleigh[0],
        rayleigh_g: tau_rayleigh[1],
        rayleigh_b: tau_rayleigh[2],
        gas_absorption_r: tau_gas_abs[0],
        gas_absorption_g: tau_gas_abs[1],
        gas_absorption_b: tau_gas_abs[2],
        dust_r: tau_ext_dust[0],
        dust_g: tau_ext_dust[1],
        dust_b: tau_ext_dust[2],
        volcanic_r: tau_ext_volc[0],
        volcanic_g: tau_ext_volc[1],
        volcanic_b: tau_ext_volc[2],
        cloud_r: tau_ext_cloud[0],
        cloud_g: tau_ext_cloud[1],
        cloud_b: tau_ext_cloud[2],
        aerosol_r: tau_aerosol[0],
        aerosol_g: tau_aerosol[1],
        aerosol_b: tau_aerosol[2],
        total_r: tau_total[0],
        total_g: tau_total[1],
        total_b: tau_total[2],
        single_scattering_albedo_r: ssa_total[0],
        single_scattering_albedo_g: ssa_total[1],
        single_scattering_albedo_b: ssa_total[2],
        asymmetry_factor_r: g_total[0],
        asymmetry_factor_g: g_total[1],
        asymmetry_factor_b: g_total[2],
    })
}

pub async fn resolve_optical_column(
    pool: &SqlitePool,
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration
) -> AppResult<SpectralOpticalDepth> {
    let atmosphere = atmosphere_repository
        ::get_by_planet_id(pool, &planet_id).await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "atmosphere".to_string(),
            reason: format!("planet '{}' has no atmosphere", planet_id),
        })?;

    let planet_row = planet_repository
        ::get_by_id(pool, &planet_id).await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let surf_temp = resolve_global_mean_temperature(
        pool,
        planet_id,
        universe_epoch,
        at_epoch
    ).await?;
    let wind_diag = resolve_wind_profile_at_latitude(
        pool,
        planet_id,
        Angle::new(0.0),
        universe_epoch,
        at_epoch
    ).await?;
    let volc_diag = resolve_planetary_volcanism(pool, planet_id, universe_epoch, at_epoch).await?;
    let hydro_opt = hydrosphere_repository::get_by_planet_id(pool, &planet_id).await?;

    let cloud_diag = resolve_cloud_cover(pool, planet_id, universe_epoch, at_epoch).await?;
    let tropo = resolve_tropopause(pool, planet_id, universe_epoch, at_epoch).await?;
    let instability = resolve_convective_instability(
        pool,
        planet_id,
        universe_epoch,
        at_epoch
    ).await?;
    let (solvent_props, _, surface_humidity) = resolve_condensable_species(pool, planet_id).await?;

    let mineralogy_diag = resolve_planetary_mineralogy(
        pool,
        planet_id,
        universe_epoch,
        at_epoch
    ).await?;
    let fe_fraction = element_mass_fraction(&mineralogy_diag.abundance.crustal_abundances, "Fe");

    let eq_radius = planet.equatorial_radius().unwrap_or_else(|| Length::new(6371e3));
    let g = surface_gravity(gravitational_parameter(planet.mass()), eq_radius);
    let scale_h = atmosphere.scale_height(g, surf_temp)?;
    let ocean_cov = hydro_opt
        .as_ref()
        .map(|h| h.surface_coverage_fraction())
        .unwrap_or(0.0);

    let bulk_shear = (
        wind_diag.jet_stream_speed.value() - wind_diag.surface_wind_speed.value()
    ).abs();
    let vertical_wind_shear = bulk_shear / tropo.tropopause_altitude.value().max(1.0);

    calculate_spectral_optical_depth(
        &atmosphere,
        &planet,
        surf_temp,
        wind_diag.surface_wind_speed,
        &volc_diag,
        ocean_cov,
        surface_humidity,
        scale_h,
        g,
        &cloud_diag,
        &instability,
        vertical_wind_shear,
        &solvent_props,
        Some(fe_fraction)
    )
}

pub async fn resolve_optical_column_at_latitude(
    pool: &SqlitePool,
    planet_id: Uuid,
    latitude: Angle,
    universe_epoch: Duration,
    at_epoch: Duration
) -> AppResult<SpectralOpticalDepth> {
    let atmosphere = atmosphere_repository
        ::get_by_planet_id(pool, &planet_id).await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "atmosphere".to_string(),
            reason: format!("planet '{}' has no atmosphere", planet_id),
        })?;

    let planet_row = planet_repository
        ::get_by_id(pool, &planet_id).await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let surf_temp = resolve_advective_surface_temperature(
        pool,
        planet_id,
        latitude,
        universe_epoch,
        at_epoch
    ).await?;
    let wind_diag = resolve_wind_profile_at_latitude(
        pool,
        planet_id,
        latitude,
        universe_epoch,
        at_epoch
    ).await?;
    let volc_diag = resolve_planetary_volcanism(pool, planet_id, universe_epoch, at_epoch).await?;
    let hydro_opt = hydrosphere_repository::get_by_planet_id(pool, &planet_id).await?;

    let cloud_diag = resolve_cloud_cover_at_latitude(
        pool,
        planet_id,
        latitude,
        universe_epoch,
        at_epoch
    ).await?;
    let tropo = resolve_tropopause_at_latitude(
        pool,
        planet_id,
        latitude,
        universe_epoch,
        at_epoch
    ).await?;
    let instability = resolve_convective_instability_at_latitude(
        pool,
        planet_id,
        latitude,
        universe_epoch,
        at_epoch
    ).await?;
    let (solvent_props, _, surface_humidity) = resolve_condensable_species(pool, planet_id).await?;

    let mineralogy_diag = resolve_planetary_mineralogy(
        pool,
        planet_id,
        universe_epoch,
        at_epoch
    ).await?;
    let fe_fraction = element_mass_fraction(&mineralogy_diag.abundance.crustal_abundances, "Fe");

    let eq_radius = planet.equatorial_radius().unwrap_or_else(|| Length::new(6371e3));
    let g = surface_gravity(gravitational_parameter(planet.mass()), eq_radius);
    let scale_h = atmosphere.scale_height(g, surf_temp)?;
    let ocean_cov = hydro_opt
        .as_ref()
        .map(|h| h.surface_coverage_fraction())
        .unwrap_or(0.0);

    let bulk_shear = (
        wind_diag.jet_stream_speed.value() - wind_diag.surface_wind_speed.value()
    ).abs();
    let vertical_wind_shear = bulk_shear / tropo.tropopause_altitude.value().max(1.0);

    calculate_spectral_optical_depth(
        &atmosphere,
        &planet,
        surf_temp,
        wind_diag.surface_wind_speed,
        &volc_diag,
        ocean_cov,
        surface_humidity,
        scale_h,
        g,
        &cloud_diag,
        &instability,
        vertical_wind_shear,
        &solvent_props,
        Some(fe_fraction)
    )
}
