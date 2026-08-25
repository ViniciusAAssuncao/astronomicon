use astronomicon_app::climate::{
    resolve_advective_surface_temperature,
    resolve_cloud_cover,
    resolve_cloud_cover_at_latitude,
    resolve_global_mean_temperature,
    resolve_star_emission_profile,
    resolve_top_of_atmosphere_irradiance,
};
use astronomicon_app::hierarchy::find_parent_star;
use astronomicon_app::sky::{
    SpectralOpticalDepth,
    SpectralRadiance,
    calculate_sky_radiances,
    linear_to_srgb,
    resolve_optical_column,
    resolve_optical_column_at_latitude,
    resolve_spectral_solar_irradiance,
};
use astronomicon_core::domain::Planet;
use astronomicon_core::math::gravity::{ gravitational_parameter, surface_gravity };
use astronomicon_core::units::{ Angle, Duration, Length, MolarMass };
use astronomicon_db::repositories::{
    atmosphere_repository,
    planet_repository,
    universe_state_repository,
};
use astronomicon_db::save::initialize_save;
use serde::{ Deserialize, Serialize };
use std::f64::consts::PI;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComprehensiveSkyReport {
    pub planet_id: Uuid,
    pub planet_name: String,
    pub planet_kind: String,
    pub star_name: String,
    pub star_kind: String,
    pub star_temperature_k: f64,
    pub star_luminosity_w: f64,
    pub mass_kg: f64,
    pub equatorial_radius_km: f64,
    pub surface_gravity_ms2: f64,
    pub surface_pressure_bar: f64,
    pub surface_pressure_pa: f64,
    pub global_mean_temperature_k: f64,
    pub global_mean_temperature_c: f64,
    pub scale_height_km: f64,
    pub mean_molar_mass_g_per_mol: f64,
    pub surface_air_density_kg_per_m3: f64,
    pub top_of_atmosphere_irradiance_w_per_m2: f64,
    pub solar_irradiance_rgb: SpectralRadianceSummary,
    pub global_optical_depth: SpectralOpticalDepthSummary,
    pub global_sky_radiance: SkyRadianceSummary,
    pub global_sky_colors: SkyColorSummary,
    pub scattering_coefficients: ScatteringCoefficientsSummary,
    pub cloud_summary: CloudSummary,
    pub latitudinal_transect: Vec<LatitudinalSkyDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectralRadianceSummary {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectralOpticalDepthSummary {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkyRadianceSummary {
    pub zenith: SpectralRadianceSummary,
    pub horizon: SpectralRadianceSummary,
    pub sunset: SpectralRadianceSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorSummary {
    pub r_srgb: f64,
    pub g_srgb: f64,
    pub b_srgb: f64,
    pub r_byte: u8,
    pub g_byte: u8,
    pub b_byte: u8,
    pub hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkyColorSummary {
    pub zenith: ColorSummary,
    pub horizon: ColorSummary,
    pub sunset: ColorSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScatteringCoefficientsSummary {
    pub rayleigh_r_per_m: f64,
    pub rayleigh_g_per_m: f64,
    pub rayleigh_b_per_m: f64,
    pub mie_r_per_m: f64,
    pub mie_g_per_m: f64,
    pub mie_b_per_m: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudSummary {
    pub total_cloud_fraction: f64,
    pub low_cloud_fraction: f64,
    pub mid_cloud_fraction: f64,
    pub high_cloud_fraction: f64,
    pub freezing_level_km: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatitudinalSkyDiagnostic {
    pub latitude_deg: f64,
    pub surface_temperature_k: f64,
    pub surface_temperature_c: f64,
    pub optical_depth: SpectralOpticalDepthSummary,
    pub sky_radiance: SkyRadianceSummary,
    pub sky_colors: SkyColorSummary,
    pub cloud_fraction: f64,
}

fn opt_depth_to_summary(od: &SpectralOpticalDepth) -> SpectralOpticalDepthSummary {
    SpectralOpticalDepthSummary {
        rayleigh_r: od.rayleigh_r,
        rayleigh_g: od.rayleigh_g,
        rayleigh_b: od.rayleigh_b,
        gas_absorption_r: od.gas_absorption_r,
        gas_absorption_g: od.gas_absorption_g,
        gas_absorption_b: od.gas_absorption_b,
        dust_r: od.dust_r,
        dust_g: od.dust_g,
        dust_b: od.dust_b,
        volcanic_r: od.volcanic_r,
        volcanic_g: od.volcanic_g,
        volcanic_b: od.volcanic_b,
        cloud_r: od.cloud_r,
        cloud_g: od.cloud_g,
        cloud_b: od.cloud_b,
        aerosol_r: od.aerosol_r,
        aerosol_g: od.aerosol_g,
        aerosol_b: od.aerosol_b,
        total_r: od.total_r,
        total_g: od.total_g,
        total_b: od.total_b,
        single_scattering_albedo_r: od.single_scattering_albedo_r,
        single_scattering_albedo_g: od.single_scattering_albedo_g,
        single_scattering_albedo_b: od.single_scattering_albedo_b,
        asymmetry_factor_r: od.asymmetry_factor_r,
        asymmetry_factor_g: od.asymmetry_factor_g,
        asymmetry_factor_b: od.asymmetry_factor_b,
    }
}

fn color_to_summary(
    radiance: SpectralRadiance,
    solar_irradiance: SpectralRadiance
) -> ColorSummary {
    let r_lin = ((PI * radiance.r) / solar_irradiance.r.max(1e-12)).clamp(0.0, 1.0);
    let g_lin = ((PI * radiance.g) / solar_irradiance.g.max(1e-12)).clamp(0.0, 1.0);
    let b_lin = ((PI * radiance.b) / solar_irradiance.b.max(1e-12)).clamp(0.0, 1.0);
    let r_srgb = linear_to_srgb(r_lin);
    let g_srgb = linear_to_srgb(g_lin);
    let b_srgb = linear_to_srgb(b_lin);
    let r_byte = (r_srgb * 255.0).round().clamp(0.0, 255.0) as u8;
    let g_byte = (g_srgb * 255.0).round().clamp(0.0, 255.0) as u8;
    let b_byte = (b_srgb * 255.0).round().clamp(0.0, 255.0) as u8;
    let hex = format!("#{:02X}{:02X}{:02X}", r_byte, g_byte, b_byte);
    ColorSummary {
        r_srgb,
        g_srgb,
        b_srgb,
        r_byte,
        g_byte,
        b_byte,
        hex,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = initialize_save("sqlite://database/astronomicon.db").await?;

    let state_row = universe_state_repository::get(&pool).await?.unwrap();
    let universe_epoch = Duration::new(state_row.seconds_since_j2000_epoch);
    let at_epoch = Duration::new(0.0);

    let planets = planet_repository::list_all(&pool).await?;

    for planet_row in planets {
        let planet_id = Uuid::parse_str(&planet_row.id)?;
        let atmosphere_opt = atmosphere_repository::get_by_planet_id(&pool, &planet_id).await?;

        if let Some(atm) = atmosphere_opt {
            let planet = Planet::try_from(planet_row)?;
            let eq_radius = planet.equatorial_radius().unwrap_or_else(|| Length::new(6371e3));
            let mu = gravitational_parameter(planet.mass());
            let g = surface_gravity(mu, eq_radius);

            let star = find_parent_star(&pool, planet.orbital_parent()).await?;
            let (star_lum, star_temp, _) = resolve_star_emission_profile(
                &pool,
                &star,
                universe_epoch,
                at_epoch
            ).await?;

            let global_temp = resolve_global_mean_temperature(
                &pool,
                planet_id,
                universe_epoch,
                at_epoch
            ).await?;
            let scale_h = atm.scale_height(g, global_temp).unwrap_or(Length::new(8500.0));
            let mean_mm = atm.mean_molar_mass().unwrap_or(MolarMass::new(0.02897));
            let surface_density = atm
                .density_at_surface(global_temp)
                .map(|d| d.value())
                .unwrap_or(1.225);

            let toa_irradiance = resolve_top_of_atmosphere_irradiance(
                &pool,
                &planet,
                &star,
                universe_epoch,
                at_epoch
            ).await?;
            let solar_irr = resolve_spectral_solar_irradiance(toa_irradiance, star_temp);

            let global_optical_depth = resolve_optical_column(
                &pool,
                planet_id,
                universe_epoch,
                at_epoch
            ).await?;
            let global_radiances = calculate_sky_radiances(&global_optical_depth, solar_irr);

            let clouds = resolve_cloud_cover(&pool, planet_id, universe_epoch, at_epoch).await?;

            let scale_h_val = scale_h.value().max(1.0);
            let scattering_summary = ScatteringCoefficientsSummary {
                rayleigh_r_per_m: global_optical_depth.rayleigh_r / scale_h_val,
                rayleigh_g_per_m: global_optical_depth.rayleigh_g / scale_h_val,
                rayleigh_b_per_m: global_optical_depth.rayleigh_b / scale_h_val,
                mie_r_per_m: global_optical_depth.aerosol_r / scale_h_val,
                mie_g_per_m: global_optical_depth.aerosol_g / scale_h_val,
                mie_b_per_m: global_optical_depth.aerosol_b / scale_h_val,
            };

            let global_colors = SkyColorSummary {
                zenith: color_to_summary(global_radiances.zenith_radiance, solar_irr),
                horizon: color_to_summary(global_radiances.horizon_radiance, solar_irr),
                sunset: color_to_summary(global_radiances.sunset_radiance, solar_irr),
            };

            let sampled_latitudes_deg = [0.0, 15.0, 30.0, 45.0, 60.0, 75.0, 90.0];
            let mut latitudinal_transect = Vec::new();

            for &lat_deg in &sampled_latitudes_deg {
                let lat_rad = Angle::new((lat_deg * PI) / 180.0);
                let surf_t = resolve_advective_surface_temperature(
                    &pool,
                    planet_id,
                    lat_rad,
                    universe_epoch,
                    at_epoch
                ).await?;
                let lat_od = resolve_optical_column_at_latitude(
                    &pool,
                    planet_id,
                    lat_rad,
                    universe_epoch,
                    at_epoch
                ).await?;
                let lat_rads = calculate_sky_radiances(&lat_od, solar_irr);
                let lat_cloud = resolve_cloud_cover_at_latitude(
                    &pool,
                    planet_id,
                    lat_rad,
                    universe_epoch,
                    at_epoch
                ).await?;

                let lat_colors = SkyColorSummary {
                    zenith: color_to_summary(lat_rads.zenith_radiance, solar_irr),
                    horizon: color_to_summary(lat_rads.horizon_radiance, solar_irr),
                    sunset: color_to_summary(lat_rads.sunset_radiance, solar_irr),
                };

                latitudinal_transect.push(LatitudinalSkyDiagnostic {
                    latitude_deg: lat_deg,
                    surface_temperature_k: surf_t.value(),
                    surface_temperature_c: surf_t.value() - 273.15,
                    optical_depth: opt_depth_to_summary(&lat_od),
                    sky_radiance: SkyRadianceSummary {
                        zenith: SpectralRadianceSummary {
                            r: lat_rads.zenith_radiance.r,
                            g: lat_rads.zenith_radiance.g,
                            b: lat_rads.zenith_radiance.b,
                        },
                        horizon: SpectralRadianceSummary {
                            r: lat_rads.horizon_radiance.r,
                            g: lat_rads.horizon_radiance.g,
                            b: lat_rads.horizon_radiance.b,
                        },
                        sunset: SpectralRadianceSummary {
                            r: lat_rads.sunset_radiance.r,
                            g: lat_rads.sunset_radiance.g,
                            b: lat_rads.sunset_radiance.b,
                        },
                    },
                    sky_colors: lat_colors,
                    cloud_fraction: lat_cloud.total_cloud_fraction,
                });
            }

            let report = ComprehensiveSkyReport {
                planet_id,
                planet_name: planet.name().to_string(),
                planet_kind: format!("{:?}", planet.kind()),
                star_name: star.name().to_string(),
                star_kind: format!("{:?}", star.kind()),
                star_temperature_k: star_temp.value(),
                star_luminosity_w: star_lum.value(),
                mass_kg: planet.mass().value(),
                equatorial_radius_km: eq_radius.value() / 1000.0,
                surface_gravity_ms2: g.value(),
                surface_pressure_bar: atm.surface_pressure().value() / 100_000.0,
                surface_pressure_pa: atm.surface_pressure().value(),
                global_mean_temperature_k: global_temp.value(),
                global_mean_temperature_c: global_temp.value() - 273.15,
                scale_height_km: scale_h.value() / 1000.0,
                mean_molar_mass_g_per_mol: mean_mm.value() * 1000.0,
                surface_air_density_kg_per_m3: surface_density,
                top_of_atmosphere_irradiance_w_per_m2: toa_irradiance.value(),
                solar_irradiance_rgb: SpectralRadianceSummary {
                    r: solar_irr.r,
                    g: solar_irr.g,
                    b: solar_irr.b,
                },
                global_optical_depth: opt_depth_to_summary(&global_optical_depth),
                global_sky_radiance: SkyRadianceSummary {
                    zenith: SpectralRadianceSummary {
                        r: global_radiances.zenith_radiance.r,
                        g: global_radiances.zenith_radiance.g,
                        b: global_radiances.zenith_radiance.b,
                    },
                    horizon: SpectralRadianceSummary {
                        r: global_radiances.horizon_radiance.r,
                        g: global_radiances.horizon_radiance.g,
                        b: global_radiances.horizon_radiance.b,
                    },
                    sunset: SpectralRadianceSummary {
                        r: global_radiances.sunset_radiance.r,
                        g: global_radiances.sunset_radiance.g,
                        b: global_radiances.sunset_radiance.b,
                    },
                },
                global_sky_colors: global_colors,
                scattering_coefficients: scattering_summary,
                cloud_summary: CloudSummary {
                    total_cloud_fraction: clouds.total_cloud_fraction,
                    low_cloud_fraction: clouds.low_cloud.cloud_fraction,
                    mid_cloud_fraction: clouds.mid_cloud.cloud_fraction,
                    high_cloud_fraction: clouds.high_cloud.cloud_fraction,
                    freezing_level_km: clouds.freezing_level.value() / 1000.0,
                },
                latitudinal_transect,
            };

            println!(
                "================================================================================"
            );
            println!(
                "DIAGNÓSTICO ÓPTICO DA ATMOSFERA E CÉU PROCEDURAL: {}",
                report.planet_name.to_uppercase()
            );
            println!(
                "================================================================================"
            );

            println!("1. PARÂMETROS FÍSICOS DO SISTEMA E DA ATMOSFERA:");
            println!("  Classificação Planetária:       {}", report.planet_kind);
            println!(
                "  Estrela Hospedeira:            {} ({})",
                report.star_name,
                report.star_kind
            );
            println!("  Temperatura Efetiva Estelar:   {:.1} K", report.star_temperature_k);
            println!("  Luminosidade Estelar:          {:.3e} W", report.star_luminosity_w);
            println!(
                "  Irradiância no Topo (TOA):     {:.2} W/m²",
                report.top_of_atmosphere_irradiance_w_per_m2
            );
            println!(
                "  Espectro Solar (R / G / B):    {:.2} / {:.2} / {:.2} W/m²",
                report.solar_irradiance_rgb.r,
                report.solar_irradiance_rgb.g,
                report.solar_irradiance_rgb.b
            );
            println!("  Massa Planetária:              {:.3e} kg", report.mass_kg);
            println!("  Raio Equatorial:               {:.2} km", report.equatorial_radius_km);
            println!("  Gravidade Superficial:         {:.2} m/s²", report.surface_gravity_ms2);
            println!(
                "  Pressão Superficial:           {:.4} bar ({:.1} Pa)",
                report.surface_pressure_bar,
                report.surface_pressure_pa
            );
            println!(
                "  Temperatura Global Média:      {:.2} K ({:.2} °C)",
                report.global_mean_temperature_k,
                report.global_mean_temperature_c
            );
            println!("  Altura de Escala (H):          {:.2} km", report.scale_height_km);
            println!(
                "  Massa Molar Média do Ar:       {:.3} g/mol",
                report.mean_molar_mass_g_per_mol
            );
            println!(
                "  Densidade Superficial do Ar:   {:.4} kg/m³",
                report.surface_air_density_kg_per_m3
            );

            println!(
                "\n2. PROFUNDIDADES ÓPTICAS VERTICAIS DA COLUNA (R: 680nm | G: 550nm | B: 440nm):"
            );
            println!(
                "  Espalhamento Rayleigh (τ_ray): {:.4} / {:.4} / {:.4}",
                report.global_optical_depth.rayleigh_r,
                report.global_optical_depth.rayleigh_g,
                report.global_optical_depth.rayleigh_b
            );
            println!(
                "  Absorção Molecular de Gases:   {:.4} / {:.4} / {:.4}",
                report.global_optical_depth.gas_absorption_r,
                report.global_optical_depth.gas_absorption_g,
                report.global_optical_depth.gas_absorption_b
            );
            println!(
                "  Aerossol de Poeira Mineral:    {:.4} / {:.4} / {:.4}",
                report.global_optical_depth.dust_r,
                report.global_optical_depth.dust_g,
                report.global_optical_depth.dust_b
            );
            println!(
                "  Aerossol Vulcânico (SO2/Ash):  {:.4} / {:.4} / {:.4}",
                report.global_optical_depth.volcanic_r,
                report.global_optical_depth.volcanic_g,
                report.global_optical_depth.volcanic_b
            );
            println!(
                "  Condensado de Nuvens:          {:.4} / {:.4} / {:.4}",
                report.global_optical_depth.cloud_r,
                report.global_optical_depth.cloud_g,
                report.global_optical_depth.cloud_b
            );
            println!(
                "  Total de Aerossóis / Partículas:{:.4} / {:.4} / {:.4}",
                report.global_optical_depth.aerosol_r,
                report.global_optical_depth.aerosol_g,
                report.global_optical_depth.aerosol_b
            );
            println!("  -------------------------------------------------------------");
            println!(
                "  Profundidade Óptica Total (τ): {:.4} / {:.4} / {:.4}",
                report.global_optical_depth.total_r,
                report.global_optical_depth.total_g,
                report.global_optical_depth.total_b
            );
            println!(
                "  Albedo de Espalhamento Único:  {:.4} / {:.4} / {:.4}",
                report.global_optical_depth.single_scattering_albedo_r,
                report.global_optical_depth.single_scattering_albedo_g,
                report.global_optical_depth.single_scattering_albedo_b
            );
            println!(
                "  Fator de Assimetria Médio (g): {:.4} / {:.4} / {:.4}",
                report.global_optical_depth.asymmetry_factor_r,
                report.global_optical_depth.asymmetry_factor_g,
                report.global_optical_depth.asymmetry_factor_b
            );

            println!("\n3. COEFICIENTES DE EXTINÇÃO E ESPALHAMENTO NA SUPERFÍCIE (1/m):");
            println!(
                "  Rayleigh (β_ray):              {:.3e} / {:.3e} / {:.3e} m⁻¹",
                report.scattering_coefficients.rayleigh_r_per_m,
                report.scattering_coefficients.rayleigh_g_per_m,
                report.scattering_coefficients.rayleigh_b_per_m
            );
            println!(
                "  Mie / Aerossóis (β_mie):       {:.3e} / {:.3e} / {:.3e} m⁻¹",
                report.scattering_coefficients.mie_r_per_m,
                report.scattering_coefficients.mie_g_per_m,
                report.scattering_coefficients.mie_b_per_m
            );

            println!("\n4. RADIÂNCIAS ESPECTRAIS EM GEOMETRIAS CANÔNICAS (W/(m²·sr)):");
            println!(
                "  Zênite (Sol a Pino):           {:.4e} / {:.4e} / {:.4e}",
                report.global_sky_radiance.zenith.r,
                report.global_sky_radiance.zenith.g,
                report.global_sky_radiance.zenith.b
            );
            println!(
                "  Horizonte (Elevação Média):    {:.4e} / {:.4e} / {:.4e}",
                report.global_sky_radiance.horizon.r,
                report.global_sky_radiance.horizon.g,
                report.global_sky_radiance.horizon.b
            );
            println!(
                "  Pôr-do-Sol (Sol Rasante):      {:.4e} / {:.4e} / {:.4e}",
                report.global_sky_radiance.sunset.r,
                report.global_sky_radiance.sunset.g,
                report.global_sky_radiance.sunset.b
            );

            println!("\n5. CORES PERCEBIDAS DO CÉU (ESPAÇO DE COR sRGB):");
            println!(
                "  Zênite:     {} | RGB: ({:>3}, {:>3}, {:>3}) | sRGB: ({:.3}, {:.3}, {:.3})",
                report.global_sky_colors.zenith.hex,
                report.global_sky_colors.zenith.r_byte,
                report.global_sky_colors.zenith.g_byte,
                report.global_sky_colors.zenith.b_byte,
                report.global_sky_colors.zenith.r_srgb,
                report.global_sky_colors.zenith.g_srgb,
                report.global_sky_colors.zenith.b_srgb
            );
            println!(
                "  Horizonte:  {} | RGB: ({:>3}, {:>3}, {:>3}) | sRGB: ({:.3}, {:.3}, {:.3})",
                report.global_sky_colors.horizon.hex,
                report.global_sky_colors.horizon.r_byte,
                report.global_sky_colors.horizon.g_byte,
                report.global_sky_colors.horizon.b_byte,
                report.global_sky_colors.horizon.r_srgb,
                report.global_sky_colors.horizon.g_srgb,
                report.global_sky_colors.horizon.b_srgb
            );
            println!(
                "  Pôr-do-Sol: {} | RGB: ({:>3}, {:>3}, {:>3}) | sRGB: ({:.3}, {:.3}, {:.3})",
                report.global_sky_colors.sunset.hex,
                report.global_sky_colors.sunset.r_byte,
                report.global_sky_colors.sunset.g_byte,
                report.global_sky_colors.sunset.b_byte,
                report.global_sky_colors.sunset.r_srgb,
                report.global_sky_colors.sunset.g_srgb,
                report.global_sky_colors.sunset.b_srgb
            );

            println!("\n6. COBERTURA DE NUVENS E MACROFÍSICA:");
            println!(
                "  Cobertura Total de Nuvens:     {:.1}%",
                report.cloud_summary.total_cloud_fraction * 100.0
            );
            println!(
                "  Camada Baixa:                  {:.1}%",
                report.cloud_summary.low_cloud_fraction * 100.0
            );
            println!(
                "  Camada Média:                  {:.1}%",
                report.cloud_summary.mid_cloud_fraction * 100.0
            );
            println!(
                "  Camada Alta:                   {:.1}%",
                report.cloud_summary.high_cloud_fraction * 100.0
            );
            println!(
                "  Nível de Congelamento:         {:.2} km",
                report.cloud_summary.freezing_level_km
            );

            println!("\n7. TABELA DE TRANSECTO LATITUDINAL DE CÉU E ÓPTICA:");
            println!(
                "--------------------------------------------------------------------------------------------------------------------------------"
            );
            println!(
                "{:<6} | {:<7} | {:<20} | {:<10} | {:<10} | {:<10} | {:<8}",
                "Lat",
                "T_surf",
                "Prof. Óptica (R/G/B)",
                "Zênite",
                "Horizonte",
                "Pôr-do-Sol",
                "Nuvens"
            );
            println!(
                "--------------------------------------------------------------------------------------------------------------------------------"
            );
            for lat in &report.latitudinal_transect {
                let tau_str = format!(
                    "{:.2}/{:.2}/{:.2}",
                    lat.optical_depth.total_r,
                    lat.optical_depth.total_g,
                    lat.optical_depth.total_b
                );
                println!(
                    "{:>4.0}° | {:>5.1} K | {:<20} | {:<10} | {:<10} | {:<10} | {:>6.1}%",
                    lat.latitude_deg,
                    lat.surface_temperature_k,
                    tau_str,
                    lat.sky_colors.zenith.hex,
                    lat.sky_colors.horizon.hex,
                    lat.sky_colors.sunset.hex,
                    lat.cloud_fraction * 100.0
                );
            }
            println!(
                "--------------------------------------------------------------------------------------------------------------------------------"
            );

            println!("\n8. DETALHAMENTO ESPECTRAL POR LATITUDE:");
            for lat in &report.latitudinal_transect {
                println!(
                    "\n  === LATITUDE {:>4.1}° (T_surf: {:.2} K / {:.2} °C | Nuvens: {:.1}%) ===",
                    lat.latitude_deg,
                    lat.surface_temperature_k,
                    lat.surface_temperature_c,
                    lat.cloud_fraction * 100.0
                );
                println!(
                    "    Profundidades Ópticas: Total=[{:.3}, {:.3}, {:.3}] | Rayleigh=[{:.3}, {:.3}, {:.3}] | Aerossol=[{:.3}, {:.3}, {:.3}]",
                    lat.optical_depth.total_r,
                    lat.optical_depth.total_g,
                    lat.optical_depth.total_b,
                    lat.optical_depth.rayleigh_r,
                    lat.optical_depth.rayleigh_g,
                    lat.optical_depth.rayleigh_b,
                    lat.optical_depth.aerosol_r,
                    lat.optical_depth.aerosol_g,
                    lat.optical_depth.aerosol_b
                );
                println!(
                    "    Albedo SSA / Assimetria g:  SSA=[{:.3}, {:.3}, {:.3}] | g=[{:.3}, {:.3}, {:.3}]",
                    lat.optical_depth.single_scattering_albedo_r,
                    lat.optical_depth.single_scattering_albedo_g,
                    lat.optical_depth.single_scattering_albedo_b,
                    lat.optical_depth.asymmetry_factor_r,
                    lat.optical_depth.asymmetry_factor_g,
                    lat.optical_depth.asymmetry_factor_b
                );
                println!(
                    "    Zênite:     {} | RGB: ({:>3}, {:>3}, {:>3})",
                    lat.sky_colors.zenith.hex,
                    lat.sky_colors.zenith.r_byte,
                    lat.sky_colors.zenith.g_byte,
                    lat.sky_colors.zenith.b_byte
                );
                println!(
                    "    Horizonte:  {} | RGB: ({:>3}, {:>3}, {:>3})",
                    lat.sky_colors.horizon.hex,
                    lat.sky_colors.horizon.r_byte,
                    lat.sky_colors.horizon.g_byte,
                    lat.sky_colors.horizon.b_byte
                );
                println!(
                    "    Pôr-do-Sol: {} | RGB: ({:>3}, {:>3}, {:>3})",
                    lat.sky_colors.sunset.hex,
                    lat.sky_colors.sunset.r_byte,
                    lat.sky_colors.sunset.g_byte,
                    lat.sky_colors.sunset.b_byte
                );
            }

            let report_path = format!("sky_diagnostics_{}.txt", report.planet_id);
            let report_json = serde_json::to_string_pretty(&report)?;
            std::fs::write(&report_path, report_json)?;
            println!("\n9. SÍNTESE DO DIAGNÓSTICO exportada para: {}\n", report_path);
        }
    }

    Ok(())
}
