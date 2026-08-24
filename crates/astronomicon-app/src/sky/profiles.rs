use crate::climate::{AtmosphericStratificationDiagnostic, CloudCoverDiagnostic};
use crate::error::AppResult;
use crate::volcanism::VolcanicDiagnostic;
use astronomicon_core::chemistry::optics::{GasOpticalProperties, mean_gas_optical_properties};
use astronomicon_core::chemistry::solvent::SolventProperties;
use astronomicon_core::domain::{Atmosphere, Planet};
use astronomicon_core::math::aerosol::{
    airborne_dust_density_with_gustiness, dust_threshold_surface_wind_with_params,
    dynamic_aerosol_scale_height, volcanic_aerosol_density,
};
use astronomicon_core::math::atmospheric_scattering::{
    CloudProfile, DustProfile, SphericalAtmosphere, VolcanicProfile,
};
use astronomicon_core::math::volcanism::VolcanicEruptionStyle;
use astronomicon_core::units::{
    Acceleration, Density, DynamicViscosity, Length, Speed, Temperature,
};

pub fn build_sky_atmosphere(
    planet: &Planet,
    atm: &Atmosphere,
    surf_temp: Temperature,
    wind_speed: Speed,
    volc_diag: &VolcanicDiagnostic,
    ocean_cov: f64,
    eq_radius: Length,
    g: Acceleration,
    cloud_diag: &CloudCoverDiagnostic,
    strat_diag: &AtmosphericStratificationDiagnostic,
    solvent_props: &SolventProperties,
    cloud_particle_radius: Length,
) -> AppResult<(
    SphericalAtmosphere,
    DustProfile,
    VolcanicProfile,
    GasOpticalProperties,
    Length,
)> {
    let atm_dens = atm.density_at_surface(surf_temp)?;
    let scale_h = atm.scale_height(g, surf_temp)?;
    let dyn_visc = atm
        .mean_dynamic_viscosity(surf_temp)
        .unwrap_or_else(|_| DynamicViscosity::new(1.81e-5));

    let z0 = planet
        .surface_roughness()
        .unwrap_or_else(|| Length::new(0.01));
    let z0_val = z0.value().clamp(1e-6, 1.0);
    let cd = (0.40 / (10.0 / z0_val).ln()).powi(2);

    let grain_density = Density::new(2650.0);
    let v_thresh =
        dust_threshold_surface_wind_with_params(g, atm_dens, grain_density, dyn_visc, Some(cd));

    let dust_availability = planet.dust_availability_factor().unwrap_or(1.0);
    let humidity = atm.surface_humidity().unwrap_or(0.0);
    let shape_parameter = 2.0;

    let dust_dens = airborne_dust_density_with_gustiness(
        wind_speed,
        v_thresh,
        atm_dens,
        g,
        dust_availability,
        ocean_cov,
        humidity,
        shape_parameter,
    );

    let dust_particle_radius = Length::new(1.0e-6);
    let dust_particle_density = Density::new(2650.0);
    let k_zz = 1.2;
    let derived_aero_h = dynamic_aerosol_scale_height(
        scale_h,
        g,
        k_zz,
        dust_particle_density,
        atm_dens,
        dust_particle_radius,
        dyn_visc,
    );

    let dust_scale_h = if derived_aero_h.value() > 0.0 {
        derived_aero_h
    } else {
        Length::new(1500.0)
    };

    let dust_profile = DustProfile::from_material(
        dust_dens,
        dust_scale_h,
        dust_particle_radius,
        dust_particle_density,
        1.55,
        0.005,
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

    let subaerial_volcanic_factor = match eruption_style {
        VolcanicEruptionStyle::Explosive => 0.20,
        VolcanicEruptionStyle::Effusive => 0.02,
        VolcanicEruptionStyle::Cryovolcanic => 0.10,
        VolcanicEruptionStyle::SubaqueousEffusive | VolcanicEruptionStyle::Inactive => 0.0,
    };

    let volc_dens = Density::new(
        volcanic_aerosol_density(
            volc_diag.outgassing_rate_sulfur,
            eruption_style,
            volc_diag.global_magma_production_rate,
            eq_radius,
            scale_h,
        )
        .value()
            * subaerial_volcanic_factor,
    );

    let (inj_alt, plume_thick) = match eruption_style {
        VolcanicEruptionStyle::Explosive => (
            Length::new(scale_h.value() * 1.8),
            Length::new(scale_h.value() * 0.4),
        ),
        VolcanicEruptionStyle::Cryovolcanic => (
            Length::new(scale_h.value() * 1.2),
            Length::new(scale_h.value() * 0.3),
        ),
        VolcanicEruptionStyle::Effusive | VolcanicEruptionStyle::SubaqueousEffusive => {
            (Length::new(0.0), Length::new(scale_h.value() * 0.6))
        }
        VolcanicEruptionStyle::Inactive => (Length::new(0.0), Length::new(1000.0)),
    };

    let volcanic_profile = VolcanicProfile::from_material(
        inj_alt,
        plume_thick,
        volc_dens,
        Length::new(5.0e-6),
        Density::new(2400.0),
        1.52,
        0.015,
    );

    let rho_cond_low = cloud_diag.low_cloud.liquid_water_content.value()
        + cloud_diag.low_cloud.ice_water_content.value();
    let rho_cond_mid = cloud_diag.mid_cloud.liquid_water_content.value()
        + cloud_diag.mid_cloud.ice_water_content.value();
    let rho_cond_high = cloud_diag.high_cloud.liquid_water_content.value()
        + cloud_diag.high_cloud.ice_water_content.value();
    let max_cond_density = rho_cond_low.max(rho_cond_mid).max(rho_cond_high);
    let cloud_base_density = Density::new(max_cond_density);

    let total_ice = cloud_diag.low_cloud.ice_water_content.value()
        + cloud_diag.mid_cloud.ice_water_content.value()
        + cloud_diag.high_cloud.ice_water_content.value();
    let total_liquid = cloud_diag.low_cloud.liquid_water_content.value()
        + cloud_diag.mid_cloud.liquid_water_content.value()
        + cloud_diag.high_cloud.liquid_water_content.value();
    let total_cond = total_ice + total_liquid;
    let avg_ice_frac = if total_cond > 0.0 {
        (total_ice / total_cond).clamp(0.0, 1.0)
    } else {
        cloud_diag.low_cloud.ice_fraction.clamp(0.0, 1.0)
    };

    let cloud_n_real = (1.0 - avg_ice_frac) * solvent_props.liquid_refractive_index_real
        + avg_ice_frac * solvent_props.solid_refractive_index_real;
    let cloud_n_imag = (1.0 - avg_ice_frac) * solvent_props.liquid_refractive_index_imag
        + avg_ice_frac * solvent_props.solid_refractive_index_imag;
    let cloud_particle_density = Density::new(
        (1.0 - avg_ice_frac) * solvent_props.liquid_density.value()
            + avg_ice_frac * solvent_props.solid_density.value(),
    );

    let cloud_profile = CloudProfile::from_material(
        cloud_base_density,
        cloud_diag.total_cloud_fraction,
        strat_diag.lcl_altitude,
        strat_diag.cloud_top_altitude,
        cloud_particle_radius,
        cloud_particle_density,
        cloud_n_real,
        cloud_n_imag,
    );

    let mut comp: Vec<(String, f64)> = atm
        .composition()
        .iter()
        .map(|c| (c.formula().to_string(), c.percentage()))
        .collect();

    let has_o2 = comp.iter().any(|(f, p)| f == "O2" && *p > 0.5);
    let has_o3 = comp.iter().any(|(f, p)| f == "O3" && *p > 1e-6);
    if has_o2 && !has_o3 {
        let o2_pct = comp
            .iter()
            .find(|(f, _)| f == "O2")
            .map(|(_, p)| *p)
            .unwrap_or(21.0);
        let o3_equiv = 3.5e-5 * (o2_pct / 21.0).sqrt();
        comp.push(("O3".to_string(), o3_equiv));
    }

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
        volcanic_profile,
    );

    Ok((
        atmosphere,
        dust_profile,
        volcanic_profile,
        opt_props,
        scale_h,
    ))
}