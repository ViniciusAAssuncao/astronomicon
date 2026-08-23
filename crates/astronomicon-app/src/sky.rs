use crate::climate::{
    find_parent_star, resolve_global_mean_temperature, resolve_star_emission_profile,
    resolve_wind_profile_at_latitude,
};
use crate::ephemeris::resolve_system_positions;
use crate::error::AppResult;
use crate::hydrosphere::resolve_hydrosphere_diagnostics;
use crate::volcanism::resolve_planetary_volcanism;
use astronomicon_core::chemistry::optics::mean_gas_optical_properties;
use astronomicon_core::domain::Planet;
use astronomicon_core::error::DomainError;
use astronomicon_core::math::aerosol::{
    airborne_dust_density, cloud_condensate_density, composite_aerosol_properties,
    dust_threshold_surface_wind, volcanic_aerosol_density,
};
use astronomicon_core::math::colorimetry::{
    exposure_tone_map, linear_to_srgb_gamma, spectral_radiance_to_xyz, xyz_to_linear_srgb,
};
use astronomicon_core::math::gravity::{gravitational_parameter, surface_gravity};
use astronomicon_core::math::optics::{
    henyey_greenstein_phase_function, mie_scattering_coefficient, molecular_number_density,
    rayleigh_phase_function_with_depolarization, rayleigh_scattering_coefficient, relative_airmass,
};
use astronomicon_core::math::radiation::planck_spectral_radiance;
use astronomicon_core::math::thermodynamics::MatterState;
use astronomicon_core::math::volcanism::VolcanicEruptionStyle;
use astronomicon_core::units::constants::OPTICAL_REFERENCE_WAVELENGTH;
use astronomicon_core::units::{
    Angle, ColorRGB, Duration, Length, SpectralRadiance, Wavelength,
};
use astronomicon_db::repositories::{atmosphere_repository, planet_repository};
use astronomicon_db::SqlitePool;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ScatteringCoefficients {
    pub rayleigh_r: f64,
    pub rayleigh_g: f64,
    pub rayleigh_b: f64,
    pub mie_r: f64,
    pub mie_g: f64,
    pub mie_b: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SkyColorDiagnostic {
    pub zenith_color: ColorRGB,
    pub horizon_color: ColorRGB,
    pub sunset_color: ColorRGB,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SkyDiagnostic {
    pub scattering: ScatteringCoefficients,
    pub colors: SkyColorDiagnostic,
    pub total_optical_depth_r: f64,
    pub total_optical_depth_g: f64,
    pub total_optical_depth_b: f64,
}

pub async fn resolve_sky_diagnostics(
    pool: &SqlitePool,
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<Option<SkyDiagnostic>> {
    let planet_row = planet_repository::get_by_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let atm_row = atmosphere_repository::get_by_planet_id(pool, &planet_id).await?;
    let atm = match atm_row {
        Some(a) => a,
        None => return Ok(None),
    };

    let star = find_parent_star(pool, &planet).await?;
    let (_, star_temp, r_emit) =
        resolve_star_emission_profile(pool, &star, universe_epoch, at_epoch).await?;

    let sys_id = star
        .star_system_id()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "star_system_id".to_string(),
            reason: "parent star is not assigned to a star system".to_string(),
        })?;
    let positions = resolve_system_positions(pool, sys_id, universe_epoch + at_epoch).await?;
    let pos_p = positions
        .get(&planet_id)
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: "planet position could not be resolved".to_string(),
        })?;
    let pos_s = positions
        .get(&star.id())
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "star_id".to_string(),
            reason: "star position could not be resolved".to_string(),
        })?;
    let distance = (*pos_p - *pos_s).magnitude();

    let solid_angle = PI * (r_emit.value() / distance.value()).powi(2);

    let surf_temp =
        resolve_global_mean_temperature(pool, planet_id, universe_epoch, at_epoch).await?;
    let wind_diag = resolve_wind_profile_at_latitude(
        pool,
        planet_id,
        Angle::new(0.0),
        universe_epoch,
        at_epoch,
    )
    .await?;
    let volc_diag = resolve_planetary_volcanism(pool, planet_id, universe_epoch, at_epoch).await?;
    let hydro_diag =
        resolve_hydrosphere_diagnostics(pool, planet_id, universe_epoch, at_epoch).await?;
    let hydro_opt = astronomicon_db::repositories::hydrosphere_repository::get_by_planet_id(pool, &planet_id).await?;

    let eq_radius = planet
        .equatorial_radius()
        .unwrap_or_else(|| Length::new(6371e3));
    let g = surface_gravity(gravitational_parameter(planet.mass()), eq_radius);
    let atm_dens = atm.density_at_surface(surf_temp)?;
    let scale_h = atm.scale_height(g, surf_temp)?;

    let v_thresh = dust_threshold_surface_wind(g, atm_dens);
    let dust_dens = airborne_dust_density(wind_diag.surface_wind_speed, v_thresh, atm_dens, g);

    let eruption_style = if volc_diag.is_cryovolcanic {
        VolcanicEruptionStyle::Cryovolcanic
    } else if volc_diag.explosive_fraction > volc_diag.effusive_fraction {
        VolcanicEruptionStyle::Explosive
    } else if volc_diag.global_magma_production_rate.value() > 0.0 {
        VolcanicEruptionStyle::Effusive
    } else {
        VolcanicEruptionStyle::Inactive
    };

    let volc_dens = volcanic_aerosol_density(
        volc_diag.outgassing_rate_sulfur,
        eruption_style,
        volc_diag.global_magma_production_rate,
        eq_radius,
        scale_h,
    );

    let matter_state = hydro_diag
        .map(|h| h.dominant_state)
        .unwrap_or(MatterState::Solid);
    let cov = hydro_opt
        .map(|h| h.surface_coverage_fraction())
        .unwrap_or(0.0);
    let cloud_dens = cloud_condensate_density(
        matter_state,
        cov,
        surf_temp,
        atm.surface_pressure(),
    );

    let aerosol_props = composite_aerosol_properties(dust_dens, volc_dens, cloud_dens);

    let comp: Vec<(String, f64)> = atm
        .composition()
        .iter()
        .map(|c| (c.formula().to_string(), c.percentage()))
        .collect();
    let opt_props = mean_gas_optical_properties(&comp)?;
    let refr_stp = opt_props.refractivity_stp();
    let king = opt_props.king_factor();
    let abs_cross = opt_props.base_absorption_cross_section_m2();

    let wl_r = Wavelength::new(680e-9);
    let wl_g = Wavelength::new(550e-9);
    let wl_b = Wavelength::new(440e-9);
    let ref_wl = Wavelength::new(OPTICAL_REFERENCE_WAVELENGTH);

    let p_surf = atm.surface_pressure();
    let n_mol = molecular_number_density(p_surf, surf_temp);
    let b_a = abs_cross * n_mol;

    let b_r_r = rayleigh_scattering_coefficient(wl_r, refr_stp, king, p_surf, surf_temp);
    let b_r_g = rayleigh_scattering_coefficient(wl_g, refr_stp, king, p_surf, surf_temp);
    let b_r_b = rayleigh_scattering_coefficient(wl_b, refr_stp, king, p_surf, surf_temp);

    let b_m_r = mie_scattering_coefficient(&aerosol_props, wl_r, ref_wl);
    let b_m_g = mie_scattering_coefficient(&aerosol_props, wl_g, ref_wl);
    let b_m_b = mie_scattering_coefficient(&aerosol_props, wl_b, ref_wl);

    let tau_r_r = b_r_r * scale_h.value();
    let tau_r_g = b_r_g * scale_h.value();
    let tau_r_b = b_r_b * scale_h.value();

    let tau_m_r = b_m_r * scale_h.value();
    let tau_m_g = b_m_g * scale_h.value();
    let tau_m_b = b_m_b * scale_h.value();

    let tau_a = b_a * scale_h.value();

    let scattering = ScatteringCoefficients {
        rayleigh_r: b_r_r,
        rayleigh_g: b_r_g,
        rayleigh_b: b_r_b,
        mie_r: b_m_r,
        mie_g: b_m_g,
        mie_b: b_m_b,
    };

    let integrate_sky = |m_in: f64, m_out: f64, theta: Angle| {
        spectral_radiance_to_xyz(|lambda| {
            let e0 = planck_spectral_radiance(lambda, star_temp) * solid_angle;
            let br = rayleigh_scattering_coefficient(lambda, refr_stp, king, p_surf, surf_temp);
            let bm = mie_scattering_coefficient(&aerosol_props, lambda, ref_wl);
            let tr = br * scale_h.value();
            let tm = bm * scale_h.value();
            let t_tot = tr + tm + tau_a;

            let pr = rayleigh_phase_function_with_depolarization(theta, king);
            let pm = henyey_greenstein_phase_function(theta, aerosol_props.asymmetry_factor_g());

            let p_tau_scat = pr * tr + pm * tm;

            let rad = if (m_in - m_out).abs() < 1e-6 {
                e0 * p_tau_scat * m_out * (-t_tot * m_out).exp()
            } else {
                e0 * p_tau_scat * (m_out / (m_in - m_out))
                    * ((-t_tot * m_out).exp() - (-t_tot * m_in).exp())
            };

            SpectralRadiance::new(rad.max(0.0))
        })
    };

    let am_zenith_sun = relative_airmass(Angle::new(PI / 4.0));
    let am_zenith_obs = relative_airmass(Angle::new(0.0));
    let xyz_zenith = integrate_sky(am_zenith_sun, am_zenith_obs, Angle::new(PI / 4.0));

    let am_horizon_sun = relative_airmass(Angle::new(0.0));
    let am_horizon_obs = relative_airmass(Angle::new(PI / 2.0));
    let xyz_horizon = integrate_sky(am_horizon_sun, am_horizon_obs, Angle::new(PI / 2.0));

    let am_sunset_sun = relative_airmass(Angle::new(PI / 2.0));
    let am_sunset_obs = relative_airmass(Angle::new(0.0));
    let xyz_sunset = integrate_sky(am_sunset_sun, am_sunset_obs, Angle::new(PI / 2.0));

    let exposure = if xyz_zenith.y() > 1e-12 {
        1.0 / xyz_zenith.y()
    } else {
        1.0
    };

    let process_color = |xyz| {
        let rgb = xyz_to_linear_srgb(xyz);
        let exposed = exposure_tone_map(rgb, exposure);
        linear_to_srgb_gamma(exposed)
    };

    let colors = SkyColorDiagnostic {
        zenith_color: process_color(xyz_zenith),
        horizon_color: process_color(xyz_horizon),
        sunset_color: process_color(xyz_sunset),
    };

    Ok(Some(SkyDiagnostic {
        scattering,
        colors,
        total_optical_depth_r: tau_r_r + tau_m_r + tau_a,
        total_optical_depth_g: tau_r_g + tau_m_g + tau_a,
        total_optical_depth_b: tau_r_b + tau_m_b + tau_a,
    }))
}