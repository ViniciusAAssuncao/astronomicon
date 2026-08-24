use crate::climate::{
    find_parent_star,
    resolve_atmospheric_stratification,
    resolve_global_mean_temperature,
    resolve_star_emission_profile,
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
    airborne_dust_density,
    cloud_condensate_density,
    derived_aerosol_scale_height,
    dust_threshold_surface_wind,
    refractivity_at_temperature_pressure,
    volcanic_aerosol_density,
};
use astronomicon_core::math::atmospheric_scattering::{
    spherical_optical_depth_segment,
    CloudProfile,
    DustProfile,
    SphericalAtmosphere,
    VolcanicProfile,
};
use astronomicon_core::math::colorimetry::{
    cie_color_matching_functions,
    linear_to_srgb_gamma,
    reinhard_extended_tone_map,
    xyz_to_linear_srgb,
    ColorXYZ,
};
use astronomicon_core::math::gravity::{ gravitational_parameter, surface_gravity };
use astronomicon_core::math::optics::{ rayleigh_scattering_coefficient, refracted_sun_direction };
use astronomicon_core::math::radiation::planck_spectral_radiance;
use astronomicon_core::math::radiometry::{ stellar_angular_radius, stellar_solid_angle };
use astronomicon_core::math::scattering::{
    multiple_scattering_spectral_radiance,
    multiple_scattering_stellar_disk_spectral_radiance,
    MultipleScatteringConfig,
};
use astronomicon_core::math::thermodynamics::MatterState;
use astronomicon_core::math::volcanism::VolcanicEruptionStyle;
use astronomicon_core::units::constants::{
    CIE_WAVELENGTH_MAX_M,
    CIE_WAVELENGTH_MIN_M,
    CIE_WAVELENGTH_STEP_M,
};
use astronomicon_core::units::{ Angle, ColorRGB, Density, Duration, Length, Vector3, Wavelength };
use astronomicon_db::repositories::{
    atmosphere_repository,
    hydrosphere_repository,
    planet_repository,
};
use astronomicon_db::SqlitePool;
use serde::{ Deserialize, Serialize };
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
    at_epoch: Duration
) -> AppResult<Option<SkyDiagnostic>> {
    let planet_row = planet_repository
        ::get_by_id(pool, &planet_id).await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let atm_row = atmosphere_repository::get_by_planet_id(pool, &planet_id).await?;
    let atm = match atm_row {
        Some(a) => a,
        None => {
            return Ok(None);
        }
    };

    let star = find_parent_star(pool, &planet).await?;
    let (_, star_temp, r_emit) = resolve_star_emission_profile(
        pool,
        &star,
        universe_epoch,
        at_epoch
    ).await?;

    let sys_id = star.star_system_id().ok_or_else(|| DomainError::InvalidInvariant {
        field: "star_system_id".to_string(),
        reason: "parent star is not assigned to a star system".to_string(),
    })?;
    let positions = resolve_system_positions(pool, sys_id, universe_epoch + at_epoch).await?;
    let pos_p = positions.get(&planet_id).ok_or_else(|| DomainError::InvalidInvariant {
        field: "planet_id".to_string(),
        reason: "planet position could not be resolved".to_string(),
    })?;
    let pos_s = positions.get(&star.id()).ok_or_else(|| DomainError::InvalidInvariant {
        field: "star_id".to_string(),
        reason: "star position could not be resolved".to_string(),
    })?;
    let distance = (*pos_p - *pos_s).magnitude();

    let star_angular_radius = stellar_angular_radius(r_emit, distance);
    let solid_angle_sun = stellar_solid_angle(star_angular_radius).value();

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
    let hydro_diag = resolve_hydrosphere_diagnostics(
        pool,
        planet_id,
        universe_epoch,
        at_epoch
    ).await?;
    let hydro_opt = hydrosphere_repository::get_by_planet_id(pool, &planet_id).await?;

    let eq_radius = planet.equatorial_radius().unwrap_or_else(|| Length::new(6371e3));
    let g = surface_gravity(gravitational_parameter(planet.mass()), eq_radius);
    let atm_dens = atm.density_at_surface(surf_temp)?;
    let scale_h = atm.scale_height(g, surf_temp)?;

    let v_thresh = dust_threshold_surface_wind(g, atm_dens);
    let dust_availability = planet.dust_availability_factor().unwrap_or(1.0);
    let ocean_cov = hydro_opt
        .as_ref()
        .map(|h| h.surface_coverage_fraction())
        .unwrap_or(0.0);
    let humidity = atm.surface_humidity().unwrap_or(0.0);

    let dust_dens = airborne_dust_density(
        wind_diag.surface_wind_speed,
        v_thresh,
        atm_dens,
        g,
        dust_availability,
        ocean_cov,
        humidity
    );

    let derived_aero_h = derived_aerosol_scale_height(g, scale_h, atm_dens);
    let dust_scale_h = if derived_aero_h.value() > 0.0 {
        derived_aero_h
    } else {
        Length::new(1500.0)
    };

    let dust_profile = DustProfile::from_material(
        dust_dens,
        dust_scale_h,
        Length::new(1.0e-6),
        Density::new(2650.0),
        1.55,
        0.005
    );

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
        scale_h
    );

    let (inj_alt, plume_thick) = match eruption_style {
        VolcanicEruptionStyle::Explosive =>
            (Length::new(scale_h.value() * 1.8), Length::new(scale_h.value() * 0.4)),
        VolcanicEruptionStyle::Cryovolcanic =>
            (Length::new(scale_h.value() * 1.2), Length::new(scale_h.value() * 0.3)),
        VolcanicEruptionStyle::Effusive | VolcanicEruptionStyle::SubaqueousEffusive =>
            (Length::new(0.0), Length::new(scale_h.value() * 0.6)),
        VolcanicEruptionStyle::Inactive => (Length::new(0.0), Length::new(1000.0)),
    };

    let volcanic_profile = VolcanicProfile::from_material(
        inj_alt,
        plume_thick,
        volc_dens,
        Length::new(5.0e-6),
        Density::new(2400.0),
        1.52,
        0.015
    );

    let matter_state = hydro_diag.map(|h| h.dominant_state).unwrap_or(MatterState::Solid);
    let cloud_cov = atm.cloud_coverage_fraction().unwrap_or(ocean_cov);
    let cloud_dens = cloud_condensate_density(
        matter_state,
        ocean_cov,
        surf_temp,
        atm.surface_pressure()
    );

    let (cloud_nr, cloud_ni, cloud_rho) = if let Some(hydro) = hydro_opt.as_ref() {
        match hydro.mean_solvent_properties() {
            Ok(props) =>
                (
                    props.liquid_refractive_index_real,
                    props.liquid_refractive_index_imag,
                    props.liquid_density,
                ),
            Err(_) => (1.333, 1.0e-8, Density::new(1000.0)),
        }
    } else {
        (1.333, 1.0e-8, Density::new(1000.0))
    };

    let (lcl, cloud_top) = match
        resolve_atmospheric_stratification(pool, planet_id, universe_epoch, at_epoch).await
    {
        Ok(strat) => (strat.lcl_altitude, strat.cloud_top_altitude),
        Err(_) => (Length::new(1000.0), Length::new(4000.0)),
    };

    let cloud_profile = CloudProfile::from_material(
        cloud_dens,
        cloud_cov,
        lcl,
        cloud_top,
        Length::new(10.0e-6),
        cloud_rho,
        cloud_nr,
        cloud_ni
    );

    let comp: Vec<(String, f64)> = atm
        .composition()
        .iter()
        .map(|c| (c.formula().to_string(), c.percentage()))
        .collect();
    let opt_props = mean_gas_optical_properties(&comp)?;

    let atmosphere = SphericalAtmosphere::new(
        eq_radius,
        Length::new(100_000.0),
        atm.surface_pressure(),
        surf_temp,
        scale_h,
        opt_props.clone(),
        dust_profile,
        cloud_profile,
        volcanic_profile
    );

    let ground_albedo = planet.bond_albedo().unwrap_or(0.15);
    let ms_config = MultipleScatteringConfig::new(
        32,
        16,
        Length::new(100_000.0),
        ground_albedo,
        1.0
    );

    let ray_origin = Vector3::new(0.0, eq_radius.value(), 0.0);
    let up = ray_origin.normalized();
    let refr = opt_props.refractivity_stp();
    let refr_actual = refractivity_at_temperature_pressure(refr, surf_temp, atm.surface_pressure());

    let view_zenith = Vector3::new(0.0, 1.0, 0.0);
    let sun_dir_day = Vector3::new((PI / 4.0).sin(), (PI / 4.0).cos(), 0.0).normalized();
    let s_refracted_day = refracted_sun_direction(sun_dir_day, up, refr_actual, scale_h, eq_radius);

    let view_horizon = Vector3::new(1.0, 0.0, 0.0);
    let view_sunset = Vector3::new(1.0, 0.0, 0.0);
    let sun_dir_sunset = Vector3::new(1.0, 0.0, 0.0);

    let step = CIE_WAVELENGTH_STEP_M;
    let mut xyz_zenith = ColorXYZ::zero();
    let mut xyz_horizon = ColorXYZ::zero();
    let mut xyz_sunset = ColorXYZ::zero();

    let mut lambda_m = CIE_WAVELENGTH_MIN_M;
    while lambda_m <= CIE_WAVELENGTH_MAX_M {
        let wavelength = Wavelength::new(lambda_m);
        let b_lambda = planck_spectral_radiance(wavelength, star_temp);
        let solar_irradiance = b_lambda * solid_angle_sun;

        if solar_irradiance > 0.0 && solar_irradiance.is_finite() {
            let res_zenith = multiple_scattering_spectral_radiance(
                ray_origin,
                view_zenith,
                sun_dir_day,
                solar_irradiance,
                wavelength,
                &atmosphere,
                &ms_config
            );
            xyz_zenith = xyz_zenith + cmf_eval(wavelength) * res_zenith.total_radiance;

            let res_horizon = multiple_scattering_spectral_radiance(
                ray_origin,
                view_horizon,
                s_refracted_day,
                solar_irradiance,
                wavelength,
                &atmosphere,
                &ms_config
            );
            xyz_horizon = xyz_horizon + cmf_eval(wavelength) * res_horizon.total_radiance;

            let res_sunset = multiple_scattering_stellar_disk_spectral_radiance(
                ray_origin,
                view_sunset,
                sun_dir_sunset,
                star_angular_radius,
                solar_irradiance,
                wavelength,
                &atmosphere,
                &ms_config,
                16,
                0.6
            );
            let sunset_radiance = res_sunset.total_radiance + b_lambda * res_sunset.transmittance;
            xyz_sunset = xyz_sunset + cmf_eval(wavelength) * sunset_radiance;
        }

        lambda_m += step;
    }

    xyz_zenith = xyz_zenith * step;
    xyz_horizon = xyz_horizon * step;
    xyz_sunset = xyz_sunset * step;

    let exposure = if xyz_zenith.y() > 1e-12 { 1.0 / xyz_zenith.y() } else { 1.0 };

    let process_color = |xyz: ColorXYZ| {
        let rgb = xyz_to_linear_srgb(xyz);
        let exposed = reinhard_extended_tone_map(rgb * exposure, 4.0);
        linear_to_srgb_gamma(exposed)
    };

    let colors = SkyColorDiagnostic {
        zenith_color: process_color(xyz_zenith),
        horizon_color: process_color(xyz_horizon),
        sunset_color: process_color(xyz_sunset),
    };

    let wl_r = Wavelength::new(680e-9);
    let wl_g = Wavelength::new(550e-9);
    let wl_b = Wavelength::new(440e-9);

    let p_surf = atm.surface_pressure();
    let refr_stp = opt_props.refractivity_stp();
    let king = opt_props.king_factor();

    let b_r_r = rayleigh_scattering_coefficient(wl_r, refr_stp, king, p_surf, surf_temp);
    let b_r_g = rayleigh_scattering_coefficient(wl_g, refr_stp, king, p_surf, surf_temp);
    let b_r_b = rayleigh_scattering_coefficient(wl_b, refr_stp, king, p_surf, surf_temp);

    let b_m_r =
        dust_profile.density_at_altitude(Length::new(0.0)).value() *
            dust_profile.scattering_coefficient_at_wavelength(wl_r) +
        cloud_profile.density_at_altitude(Length::new(0.0)).value() *
            cloud_profile.scattering_coefficient_at_wavelength(wl_r) +
        volcanic_profile.density_at_altitude(Length::new(0.0)).value() *
            volcanic_profile.scattering_coefficient_at_wavelength(wl_r);

    let b_m_g =
        dust_profile.density_at_altitude(Length::new(0.0)).value() *
            dust_profile.scattering_coefficient_at_wavelength(wl_g) +
        cloud_profile.density_at_altitude(Length::new(0.0)).value() *
            cloud_profile.scattering_coefficient_at_wavelength(wl_g) +
        volcanic_profile.density_at_altitude(Length::new(0.0)).value() *
            volcanic_profile.scattering_coefficient_at_wavelength(wl_g);

    let b_m_b =
        dust_profile.density_at_altitude(Length::new(0.0)).value() *
            dust_profile.scattering_coefficient_at_wavelength(wl_b) +
        cloud_profile.density_at_altitude(Length::new(0.0)).value() *
            cloud_profile.scattering_coefficient_at_wavelength(wl_b) +
        volcanic_profile.density_at_altitude(Length::new(0.0)).value() *
            volcanic_profile.scattering_coefficient_at_wavelength(wl_b);

    let scattering = ScatteringCoefficients {
        rayleigh_r: b_r_r,
        rayleigh_g: b_r_g,
        rayleigh_b: b_r_b,
        mie_r: b_m_r,
        mie_g: b_m_g,
        mie_b: b_m_b,
    };

    let vertical_top = Vector3::new(0.0, atmosphere.atmosphere_top_radius.value(), 0.0);
    let vertical_depth = spherical_optical_depth_segment(ray_origin, vertical_top, &atmosphere, 64);

    let tau_tot_r = vertical_depth.total_extinction_optical_depth(wl_r, &atmosphere);
    let tau_tot_g = vertical_depth.total_extinction_optical_depth(wl_g, &atmosphere);
    let tau_tot_b = vertical_depth.total_extinction_optical_depth(wl_b, &atmosphere);

    Ok(
        Some(SkyDiagnostic {
            scattering,
            colors,
            total_optical_depth_r: tau_tot_r,
            total_optical_depth_g: tau_tot_g,
            total_optical_depth_b: tau_tot_b,
        })
    )
}

fn cmf_eval(wavelength: Wavelength) -> ColorXYZ {
    cie_color_matching_functions(wavelength)
}
