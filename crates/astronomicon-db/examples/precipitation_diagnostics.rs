use astronomicon_app::climate::{
    AtmosphericStratificationDiagnostic, CloudCoverDiagnostic, CloudLayerDiagnostic,
    ConvectiveInstabilityDiagnostic, PlanetaryCirculationDiagnostic, PrecipitationDiagnostic,
    SevereWeatherDiagnostic, TropopauseDiagnostic, WindProfileDiagnostic,
    resolve_advective_surface_temperature, resolve_all_condensable_species,
    resolve_atmospheric_stratification_at_latitude, resolve_cloud_cover_at_latitude,
    resolve_condensable_species, resolve_convective_instability_at_latitude,
    resolve_global_mean_temperature, resolve_planetary_circulation,
    resolve_precipitation_diagnostic, resolve_severe_weather,
    resolve_tropopause_at_latitude, resolve_wind_profile_at_latitude,
};
use astronomicon_core::chemistry::solvent::SolventProperties;
use astronomicon_core::domain::Planet;
use astronomicon_core::math::clouds::{
    AtmosphericStability, CloudMorphology, GlaciationState, LightningPotential, StormMode,
};
use astronomicon_core::math::gravity::{gravitational_parameter, surface_gravity};
use astronomicon_core::math::precipitation::{
    AcidityClassification, CondensatePrimaryClass, PrecipitationPhase, SurfaceCondensationType,
};
use astronomicon_core::units::{Angle, Duration, Length, MolarMass};
use astronomicon_db::repositories::{
    atmosphere_repository, hydrosphere_repository, planet_repository,
    universe_state_repository,
};
use astronomicon_db::save::initialize_save;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComprehensivePrecipitationReport {
    pub planet_id: Uuid,
    pub planet_name: String,
    pub planet_kind: String,
    pub mass_kg: f64,
    pub equatorial_radius_km: f64,
    pub surface_gravity_ms2: f64,
    pub surface_pressure_bar: f64,
    pub surface_pressure_pa: f64,
    pub global_mean_temperature_k: f64,
    pub global_mean_temperature_c: f64,
    pub greenhouse_effect_k: f64,
    pub scale_height_km: f64,
    pub mean_molar_mass_g_per_mol: f64,
    pub surface_air_density_kg_per_m3: f64,
    pub atmospheric_composition: Vec<GasCompositionSummary>,
    pub hydrosphere_summary: Option<HydrosphereSummary>,
    pub condensable_species_inventory: Vec<CondensableSpeciesSummary>,
    pub dominant_solvent: CondensableSpeciesSummary,
    pub circulation: PlanetaryCirculationDiagnostic,
    pub severe_weather: SevereWeatherDiagnostic,
    pub latitudinal_transect: Vec<LatitudinalPrecipitationDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasCompositionSummary {
    pub formula: String,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HydrosphereSummary {
    pub coverage_fraction: f64,
    pub average_depth_m: f64,
    pub salinity_fraction: f64,
    pub composition: Vec<GasCompositionSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CondensableSpeciesSummary {
    pub formula_or_name: String,
    pub molar_mass_g_per_mol: f64,
    pub fraction: f64,
    pub enthalpy_of_vaporization_j_per_mol: f64,
    pub enthalpy_of_fusion_j_per_mol: f64,
    pub normal_boiling_point_k: f64,
    pub normal_melting_point_k: f64,
    pub triple_point_temperature_k: f64,
    pub triple_point_pressure_pa: f64,
    pub critical_temperature_k: f64,
    pub critical_pressure_pa: f64,
    pub liquid_density_kg_per_m3: f64,
    pub solid_density_kg_per_m3: f64,
    pub solid_thermal_conductivity: f64,
    pub liquid_specific_heat_capacity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatitudinalPrecipitationDiagnostic {
    pub latitude_deg: f64,
    pub surface_temperature_k: f64,
    pub surface_temperature_c: f64,
    pub tropopause: TropopauseDiagnostic,
    pub stratification: AtmosphericStratificationDiagnostic,
    pub convective_instability: ConvectiveInstabilityDiagnostic,
    pub cloud_cover: CloudCoverDiagnostic,
    pub wind_profile: WindProfileDiagnostic,
    pub precipitation: PrecipitationDiagnostic,
    pub rain_rate_mm_per_h: f64,
    pub rain_rate_mm_per_day: f64,
    pub rain_rate_m_per_year: f64,
    pub mass_flux_g_per_m2_h: f64,
}

fn format_phase(phase: PrecipitationPhase) -> &'static str {
    match phase {
        PrecipitationPhase::Liquid => "Líquida (Chuva / Chuvisco)",
        PrecipitationPhase::Solid => "Sólida (Neve / Pelotas / Cristais de Gelo)",
        PrecipitationPhase::Mixed => "Mista (Chuva Congelante / Sleet / Granizo)",
    }
}

fn format_primary_class(class: CondensatePrimaryClass) -> &'static str {
    match class {
        CondensatePrimaryClass::AqueousMolecular => "Molecular Aquosa (H2O)",
        CondensatePrimaryClass::CryogenicHydrocarbon => "Hidrocarboneto Criogênico (CH4, C2H6)",
        CondensatePrimaryClass::StrongAcid => "Ácido Forte / Corrosivo (H2SO4, HNO3, HCl)",
        CondensatePrimaryClass::OtherCovalent => "Covalente Volátil / Outro (NH3, N2, CO2)",
    }
}

fn format_acidity(acidity: AcidityClassification) -> &'static str {
    match acidity {
        AcidityClassification::Neutral => "Neutra (pH ~ 7.0)",
        AcidityClassification::NaturalBaseline => "Linha de Base Natural do Planeta (Equilíbrio CO2)",
        AcidityClassification::AcidicRelativeToBaseline => "Ácida em Relação ao Baseline Natural",
        AcidityClassification::StronglyCorrosive => "Fortemente Corrosiva / Ácido Concentrado",
    }
}

fn format_surface_condensation(cond: SurfaceCondensationType) -> &'static str {
    match cond {
        SurfaceCondensationType::Dew => "Orvalho (Condensação Líquida Superficial)",
        SurfaceCondensationType::Frost => "Geada (Deposição Sólida / Cristais Superficiais)",
    }
}

fn format_stability(stability: AtmosphericStability) -> &'static str {
    match stability {
        AtmosphericStability::AbsolutelyUnstable => "Absolutamente Instável (Superadiabática)",
        AtmosphericStability::ConditionallyUnstable => "Condicionalmente Instável",
        AtmosphericStability::AbsolutelyStable => "Absolutamente Estável",
    }
}

fn format_morphology(morphology: CloudMorphology) -> &'static str {
    match morphology {
        CloudMorphology::Convective => "Convectiva (Cúmulos / Cumulonimbos)",
        CloudMorphology::Stratiform => "Estratiforme (Nuvens em Camadas)",
    }
}

fn format_glaciation(glaciation: GlaciationState) -> &'static str {
    match glaciation {
        GlaciationState::Liquid => "Líquido (Gotículas)",
        GlaciationState::MixedPhase => "Fase Mista (Líquido + Gelo)",
        GlaciationState::Glaciated => "Glaciado (Cristais de Gelo)",
    }
}

fn format_storm_mode(mode: StormMode) -> &'static str {
    match mode {
        StormMode::None => "Nenhum Modo Organizado (Atmosfera Estável / Subcrítica)",
        StormMode::SingleCell => "Célula Única (Pulso Convectivo Ordinário)",
        StormMode::Multicell => "Multicelular (Linhas / Aglomerados Convectivos)",
        StormMode::Supercell => "Supercélula (Convecção Rotacional Severa)",
    }
}

fn format_lightning(potential: LightningPotential) -> &'static str {
    match potential {
        LightningPotential::None => "Nulo / Improvável",
        LightningPotential::Possible => "Possível (Eletrificação Moderada)",
        LightningPotential::Probable => "Provável (Alta Atividade de Descargas Elétricas)",
    }
}

fn build_condensable_species_summary(
    name: &str,
    props: &SolventProperties,
    mm: MolarMass,
    fraction: f64,
) -> CondensableSpeciesSummary {
    CondensableSpeciesSummary {
        formula_or_name: name.to_string(),
        molar_mass_g_per_mol: mm.value() * 1000.0,
        fraction,
        enthalpy_of_vaporization_j_per_mol: props.enthalpy_of_vaporization,
        enthalpy_of_fusion_j_per_mol: props.enthalpy_of_fusion,
        normal_boiling_point_k: props.normal_boiling_point.value(),
        normal_melting_point_k: props.normal_melting_point.value(),
        triple_point_temperature_k: props.triple_point_temperature.value(),
        triple_point_pressure_pa: props.triple_point_pressure.value(),
        critical_temperature_k: props.critical_temperature.value(),
        critical_pressure_pa: props.critical_pressure.value(),
        liquid_density_kg_per_m3: props.liquid_density.value(),
        solid_density_kg_per_m3: props.solid_density.value(),
        solid_thermal_conductivity: props.solid_thermal_conductivity,
        liquid_specific_heat_capacity: props.liquid_specific_heat_capacity,
    }
}

fn print_cloud_layer_details(name: &str, layer: &CloudLayerDiagnostic) {
    println!("    * Camada {}:", name);
    println!(
        "        Faixa de Altitude:      {:.2} km -> {:.2} km (Ponto Médio: {:.2} km)",
        layer.base_altitude.value() / 1000.0,
        layer.top_altitude.value() / 1000.0,
        layer.representative_altitude.value() / 1000.0
    );
    println!(
        "        Cobertura Fracionária:  {:.1}%",
        layer.cloud_fraction * 100.0
    );
    println!(
        "        Umidade Relativa (RH):  {:.1}% (RH Crítica: {:.1}%)",
        layer.relative_humidity * 100.0,
        layer.critical_relative_humidity * 100.0
    );
    println!(
        "        Água Líquida (LWC):     {:.4e} kg/m³ ({:.3} g/m³)",
        layer.liquid_water_content.value(),
        layer.liquid_water_content.value() * 1000.0
    );
    println!(
        "        Gelo (IWC):             {:.4e} kg/m³ ({:.3} g/m³)",
        layer.ice_water_content.value(),
        layer.ice_water_content.value() * 1000.0
    );
    println!(
        "        Fração de Gelo / Fase:  {:.1}% | {}",
        layer.ice_fraction * 100.0,
        format_glaciation(layer.glaciation_state)
    );
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
            let eq_radius = planet
                .equatorial_radius()
                .unwrap_or_else(|| Length::new(6371e3));
            let mu = gravitational_parameter(planet.mass());
            let g = surface_gravity(mu, eq_radius);

            let global_temp =
                resolve_global_mean_temperature(&pool, planet_id, universe_epoch, at_epoch).await?;
            let scale_h = atm
                .scale_height(g, global_temp)
                .unwrap_or(Length::new(8500.0));
            let mean_mm = atm
                .mean_molar_mass()
                .unwrap_or(MolarMass::new(0.02897));
            let surface_density = atm
                .density_at_surface(global_temp)
                .map(|d| d.value())
                .unwrap_or(1.225);

            let (dom_props, dom_mm, dom_hum) =
                resolve_condensable_species(&pool, planet_id).await?;
            let all_condensables = resolve_all_condensable_species(&pool, planet_id).await?;
            let hydro_opt = hydrosphere_repository::get_by_planet_id(&pool, &planet_id).await?;

            let circulation =
                resolve_planetary_circulation(&pool, planet_id, universe_epoch, at_epoch).await?;
            let severe_weather =
                resolve_severe_weather(&pool, planet_id, universe_epoch, at_epoch).await?;

            let composition_summaries: Vec<GasCompositionSummary> = atm
                .composition()
                .iter()
                .map(|c| GasCompositionSummary {
                    formula: c.formula().to_string(),
                    percentage: c.percentage(),
                })
                .collect();

            let hydrosphere_summary = hydro_opt.as_ref().map(|h| HydrosphereSummary {
                coverage_fraction: h.surface_coverage_fraction(),
                average_depth_m: h.average_depth().value(),
                salinity_fraction: h.salinity_or_solute_mass_fraction(),
                composition: h
                    .composition()
                    .iter()
                    .map(|c| GasCompositionSummary {
                        formula: c.formula().to_string(),
                        percentage: c.percentage(),
                    })
                    .collect(),
            });

            let mut condensable_inventory = Vec::new();
            for (idx, (props, mm, frac)) in all_condensables.iter().enumerate() {
                let name = format!("Espécie #{} (Fração Molar: {:.2}%)", idx + 1, frac * 100.0);
                condensable_inventory.push(build_condensable_species_summary(
                    &name, props, *mm, *frac,
                ));
            }

            let dominant_summary = build_condensable_species_summary(
                "Solvente Dominante",
                &dom_props,
                dom_mm,
                dom_hum,
            );

            let sampled_latitudes_deg = [0.0, 15.0, 30.0, 45.0, 60.0, 75.0, 90.0];
            let mut latitudinal_transect = Vec::new();

            for &lat_deg in &sampled_latitudes_deg {
                let lat_rad = Angle::new(lat_deg * PI / 180.0);
                let surf_t = resolve_advective_surface_temperature(
                    &pool,
                    planet_id,
                    lat_rad,
                    universe_epoch,
                    at_epoch,
                )
                .await?;
                let tropo = resolve_tropopause_at_latitude(
                    &pool,
                    planet_id,
                    lat_rad,
                    universe_epoch,
                    at_epoch,
                )
                .await?;
                let strat = resolve_atmospheric_stratification_at_latitude(
                    &pool,
                    planet_id,
                    lat_rad,
                    universe_epoch,
                    at_epoch,
                )
                .await?;
                let convective = resolve_convective_instability_at_latitude(
                    &pool,
                    planet_id,
                    lat_rad,
                    universe_epoch,
                    at_epoch,
                )
                .await?;
                let cloud_cover = resolve_cloud_cover_at_latitude(
                    &pool,
                    planet_id,
                    lat_rad,
                    universe_epoch,
                    at_epoch,
                )
                .await?;
                let wind_prof = resolve_wind_profile_at_latitude(
                    &pool,
                    planet_id,
                    lat_rad,
                    universe_epoch,
                    at_epoch,
                )
                .await?;
                let precip = resolve_precipitation_diagnostic(
                    &pool,
                    planet_id,
                    lat_rad,
                    universe_epoch,
                    at_epoch,
                )
                .await?;

                let rain_mm_h = precip.linear_accumulation_rate.value() * 1000.0 * 3600.0;
                let rain_mm_day = precip.linear_accumulation_rate.value() * 1000.0 * 86400.0;
                let rain_m_yr = precip.linear_accumulation_rate.value() * 31557600.0;
                let mass_flux_g_h = precip.mass_flux.value() * 1000.0 * 3600.0;

                latitudinal_transect.push(LatitudinalPrecipitationDiagnostic {
                    latitude_deg: lat_deg,
                    surface_temperature_k: surf_t.value(),
                    surface_temperature_c: surf_t.value() - 273.15,
                    tropopause: tropo,
                    stratification: strat,
                    convective_instability: convective,
                    cloud_cover,
                    wind_profile: wind_prof,
                    precipitation: precip,
                    rain_rate_mm_per_h: rain_mm_h,
                    rain_rate_mm_per_day: rain_mm_day,
                    rain_rate_m_per_year: rain_m_yr,
                    mass_flux_g_per_m2_h: mass_flux_g_h,
                });
            }

            let report = ComprehensivePrecipitationReport {
                planet_id,
                planet_name: planet.name().to_string(),
                planet_kind: format!("{:?}", planet.kind()),
                mass_kg: planet.mass().value(),
                equatorial_radius_km: eq_radius.value() / 1000.0,
                surface_gravity_ms2: g.value(),
                surface_pressure_bar: atm.surface_pressure().value() / 100_000.0,
                surface_pressure_pa: atm.surface_pressure().value(),
                global_mean_temperature_k: global_temp.value(),
                global_mean_temperature_c: global_temp.value() - 273.15,
                greenhouse_effect_k: atm.greenhouse_effect().value(),
                scale_height_km: scale_h.value() / 1000.0,
                mean_molar_mass_g_per_mol: mean_mm.value() * 1000.0,
                surface_air_density_kg_per_m3: surface_density,
                atmospheric_composition: composition_summaries,
                hydrosphere_summary,
                condensable_species_inventory: condensable_inventory,
                dominant_solvent: dominant_summary,
                circulation,
                severe_weather,
                latitudinal_transect,
            };

            println!(
                "================================================================================"
            );
            println!(
                "RELATÓRIO COMPLETO DE PRECIPITAÇÃO E DINÂMICA ATMOSFÉRICA: {}",
                report.planet_name.to_uppercase()
            );
            println!(
                "================================================================================"
            );

            println!("1. PROPRIEDADES FÍSICAS E ESTRUTURA PLANETÁRIA:");
            println!("  Classificação Planetária:       {}", report.planet_kind);
            println!("  Massa Planetária:              {:.3e} kg", report.mass_kg);
            println!(
                "  Raio Equatorial:               {:.2} km",
                report.equatorial_radius_km
            );
            println!(
                "  Gravidade Superficial:         {:.2} m/s²",
                report.surface_gravity_ms2
            );
            println!(
                "  Pressão Superficial:           {:.4} bar ({:.1} Pa)",
                report.surface_pressure_bar, report.surface_pressure_pa
            );
            println!(
                "  Temperatura Global Média:      {:.2} K ({:.2} °C)",
                report.global_mean_temperature_k, report.global_mean_temperature_c
            );
            println!(
                "  Efeito Estufa Atmosférico:     {:.2} K",
                report.greenhouse_effect_k
            );
            println!(
                "  Altura de Escala Atmosférica:  {:.2} km",
                report.scale_height_km
            );
            println!(
                "  Massa Molar Média do Ar:       {:.3} g/mol",
                report.mean_molar_mass_g_per_mol
            );
            println!(
                "  Densidade do Ar à Superfície:  {:.4} kg/m³",
                report.surface_air_density_kg_per_m3
            );

            println!("\n2. COMPOSIÇÃO QUÍMICA DA ATMOSFERA:");
            for comp in &report.atmospheric_composition {
                println!("  - {:<8}: {:>6.2}%", comp.formula, comp.percentage);
            }

            println!("\n3. HIDROSFERA E INVENTÁRIO DE ESPÉCIES CONDENSÁVEIS:");
            if let Some(ref hydro) = report.hydrosphere_summary {
                println!(
                    "  Cobertura Oceânica Superficial:{:.1}%",
                    hydro.coverage_fraction * 100.0
                );
                println!(
                    "  Profundidade Média dos Oceanos:{:.2} m",
                    hydro.average_depth_m
                );
                println!(
                    "  Salinidade / Solutos Totais:   {:.2}%",
                    hydro.salinity_fraction * 100.0
                );
                println!("  Composição Química da Hidrosfera:");
                for c in &hydro.composition {
                    println!("    * {:<8}: {:>6.2}%", c.formula, c.percentage);
                }
            } else {
                println!(
                    "  Hidrosfera Superficial:        Não Detectada (Atmosfera Seca/Condensação Pura)"
                );
            }

            println!("  Espécies Condensáveis Detectadas (Ordenadas por Proximidade de Saturação):");
            for (idx, sp) in report.condensable_species_inventory.iter().enumerate() {
                println!(
                    "    [{}] {} - Massa Molar: {:.2} g/mol | Ebulição: {:.2} K | Fusão: {:.2} K | ΔHvap: {:.1} J/mol",
                    idx + 1,
                    sp.formula_or_name,
                    sp.molar_mass_g_per_mol,
                    sp.normal_boiling_point_k,
                    sp.normal_melting_point_k,
                    sp.enthalpy_of_vaporization_j_per_mol
                );
            }

            println!("\n4. CIRCULAÇÃO GLOBAL E CONDIÇÕES SEVERAS:");
            println!(
                "  Velocidade Angular Planetária: {:.4e} rad/s",
                report.circulation.angular_velocity.value()
            );
            println!(
                "  Células Convectivas / Hemisf.: {}",
                report.circulation.circulation_cells
            );
            println!(
                "  Raio de Deformação de Rossby:  {:.1} km",
                report.circulation.rossby_deformation_radius.value() / 1000.0
            );
            println!(
                "  Escala de Rhines:              {:.1} km",
                report.circulation.rhines_scale.value() / 1000.0
            );
            println!(
                "  Eficiência de Redistribuição:  {:.2}%",
                report.circulation.thermal_redistribution_efficiency * 100.0
            );
            println!(
                "  Cisalhamento de Vento (Shear): {:.2} m/s ({:.1} km/h)",
                report.severe_weather.bulk_wind_shear.value(),
                report.severe_weather.bulk_wind_shear.value() * 3.6
            );
            println!(
                "  Número de Richardson (BRN):    {:.2}",
                report.severe_weather.bulk_richardson_number
            );
            println!(
                "  Modo de Tempestade Predominante:{}",
                format_storm_mode(report.severe_weather.storm_mode)
            );
            println!(
                "  Potencial de Descargas:        {}",
                format_lightning(report.severe_weather.lightning_potential)
            );
            println!(
                "  Intensidade Ciclones Máx (PI): {:.2} m/s ({:.1} km/h)",
                report
                    .severe_weather
                    .tropical_cyclone_potential_intensity
                    .value(),
                report
                    .severe_weather
                    .tropical_cyclone_potential_intensity
                    .value()
                    * 3.6
            );
            println!(
                "  Ciclogênese Tropical Favitada: {}",
                if report.severe_weather.is_cyclogenesis_favorable {
                    "Sim (Termodinâmica + Coriolis Adequados)"
                } else {
                    "Não (Inibição Térmica/Falta de Oceano/Coriolis Fraco)"
                }
            );

            println!("\n5. TABELA DE TRANSECTO LATITUDINAL DE PRECIPITAÇÃO:");
            println!("----------------------------------------------------------------------------------------------------------------------------------");
            println!(
                "{:<6} | {:<7} | {:<7} | {:<7} | {:<12} | {:<10} | {:<12} | {:<10} | {:<10} | {:<14} | {:<6}",
                "Lat", "T_surf", "P_orv", "LCL", "Fase", "Fluxo(g/m²h)", "Acum(mm/h)", "Acum(m/ano)", "Alcança?", "Cond.Superf.", "pH"
            );
            println!("----------------------------------------------------------------------------------------------------------------------------------");
            for lat in &report.latitudinal_transect {
                let reaches_str = if lat.precipitation.reaches_surface {
                    "SIM"
                } else if lat.precipitation.mass_flux.value() > 0.0 {
                    "VIRGA"
                } else {
                    "SECO"
                };
                let ph_str = match lat.precipitation.ph {
                    Some(ph_val) => format!("{:.2}", ph_val),
                    None => "N/A".to_string(),
                };
                let phase_short = match lat.precipitation.phase {
                    PrecipitationPhase::Liquid => "Líquido",
                    PrecipitationPhase::Solid => "Sólido",
                    PrecipitationPhase::Mixed => "Misto",
                };
                let cond_short = match lat.precipitation.surface_condensation {
                    SurfaceCondensationType::Dew => "Orvalho",
                    SurfaceCondensationType::Frost => "Geada",
                };

                println!(
                    "{:>4.0}° | {:>5.1} K | {:>5.1} K | {:>5.2}km | {:<12} | {:>10.2} | {:>12.4} | {:>10.4} | {:<10} | {:<14} | {:<6}",
                    lat.latitude_deg,
                    lat.surface_temperature_k,
                    lat.stratification.surface_dew_point.value(),
                    lat.stratification.lcl_altitude.value() / 1000.0,
                    phase_short,
                    lat.mass_flux_g_per_m2_h,
                    lat.rain_rate_mm_per_h,
                    lat.rain_rate_m_per_year,
                    reaches_str,
                    cond_short,
                    ph_str
                );
            }
            println!("----------------------------------------------------------------------------------------------------------------------------------");

            println!("\n6. DIAGNÓSTICO DETALHADO POR LATITUDE:");
            for lat in &report.latitudinal_transect {
                println!(
                    "\n  === LATITUDE {:>4.1}° (T_surf: {:.2} K / {:.2} °C) ===",
                    lat.latitude_deg, lat.surface_temperature_k, lat.surface_temperature_c
                );
                println!(
                    "    Estrutura Vertical da Coluna:"
                );
                println!(
                    "      Tropopausa:               {:.2} km (T_tropo: {:.2} K | T_skin: {:.2} K)",
                    lat.tropopause.tropopause_altitude.value() / 1000.0,
                    lat.tropopause.tropopause_temperature.value(),
                    lat.tropopause.skin_temperature.value()
                );
                println!(
                    "      Lapse Rate Ambiental:     {:.2} K/km",
                    lat.tropopause.lapse_rate.value() * 1000.0
                );
                println!(
                    "      Ponto de Orvalho:         {:.2} K ({:.2} °C)",
                    lat.stratification.surface_dew_point.value(),
                    lat.stratification.surface_dew_point.value() - 273.15
                );
                println!(
                    "      Nível de Condensação(LCL):{:.2} km",
                    lat.stratification.lcl_altitude.value() / 1000.0
                );
                println!(
                    "      Topo Térmico de Nuvem:    {:.2} km",
                    lat.stratification.cloud_top_altitude.value() / 1000.0
                );
                println!(
                    "      Nível de Congelamento:    {:.2} km",
                    lat.cloud_cover.freezing_level.value() / 1000.0
                );

                println!("    Termodinâmica Convectiva:");
                println!(
                    "      Lapse Rate Seco (Γd):     {:.2} K/km",
                    lat.convective_instability
                        .dry_adiabatic_lapse_rate
                        .value()
                        * 1000.0
                );
                println!(
                    "      Lapse Rate Úmido (Γm):    {:.2} K/km",
                    lat.convective_instability
                        .moist_adiabatic_lapse_rate
                        .value()
                        * 1000.0
                );
                println!(
                    "      Estabilidade Atmosférica: {}",
                    format_stability(lat.convective_instability.stability)
                );
                println!(
                    "      Morfologia de Nuvens:     {}",
                    format_morphology(lat.convective_instability.morphology)
                );
                println!(
                    "      CAPE (Energia Convectiva):{:.1} J/kg",
                    lat.convective_instability.cape.value()
                );
                println!(
                    "      CIN (Inibição Convectiva):{:.1} J/kg",
                    lat.convective_instability.cin.value()
                );
                println!(
                    "      LFC (Livre Convecção):    {}",
                    match lat.convective_instability.lfc_altitude {
                        Some(z) => format!("{:.2} km", z.value() / 1000.0),
                        None => "Inexistente".to_string(),
                    }
                );
                println!(
                    "      EL (Nível de Equilíbrio): {}",
                    match lat.convective_instability.equilibrium_level {
                        Some(z) => format!("{:.2} km", z.value() / 1000.0),
                        None => "Inexistente".to_string(),
                    }
                );
                println!(
                    "      Convecção Profunda Bigor: {}",
                    if lat.convective_instability.is_deep_convection {
                        "Sim"
                    } else {
                        "Não"
                    }
                );

                println!("    Nebulosidade e Nuvens:");
                println!(
                    "      Cobertura Total:          {:.1}%",
                    lat.cloud_cover.total_cloud_fraction * 100.0
                );
                print_cloud_layer_details("Baixa", &lat.cloud_cover.low_cloud);
                print_cloud_layer_details("Média", &lat.cloud_cover.mid_cloud);
                print_cloud_layer_details("Alta", &lat.cloud_cover.high_cloud);

                println!("    Vento e Dinâmica Local:");
                println!(
                    "      Vento Superficial:        {:.2} m/s ({:.1} km/h)",
                    lat.wind_profile.surface_wind_speed.value(),
                    lat.wind_profile.surface_wind_speed.value() * 3.6
                );
                println!(
                    "      Corrente de Jato:         {:.2} m/s ({:.1} km/h)",
                    lat.wind_profile.jet_stream_speed.value(),
                    lat.wind_profile.jet_stream_speed.value() * 3.6
                );
                println!(
                    "      Parâmetro de Coriolis (f):{:.3e} rad/s",
                    lat.wind_profile.coriolis_parameter.value()
                );

                println!("    Precipitação e Microfísica:");
                println!(
                    "      Fase Hidrometeoro:        {}",
                    format_phase(lat.precipitation.phase)
                );
                println!(
                    "      Classe do Condensado:     {}",
                    format_primary_class(lat.precipitation.primary_class)
                );
                println!(
                    "      Fluxo de Massa:           {:.4e} kg/(m²·s) ({:.2} g/(m²·h))",
                    lat.precipitation.mass_flux.value(),
                    lat.mass_flux_g_per_m2_h
                );
                println!(
                    "      Taxa Linear de Acúmulo:   {:.4e} m/s ({:.3} mm/h | {:.2} mm/dia | {:.3} m/ano)",
                    lat.precipitation.linear_accumulation_rate.value(),
                    lat.rain_rate_mm_per_h,
                    lat.rain_rate_mm_per_day,
                    lat.rain_rate_m_per_year
                );
                println!(
                    "      Alcança a Superfície:     {}",
                    if lat.precipitation.reaches_surface {
                        "SIM (Precipitação Efetiva no Solo)"
                    } else if lat.precipitation.mass_flux.value() > 0.0 {
                        "NÃO (VIRGA - Evaporação / Sublimação Sub-nuvem Completa)"
                    } else {
                        "NÃO (Ausência de Condensação Sedimentável)"
                    }
                );
                println!(
                    "      Classificação de Acidez:  {}",
                    format_acidity(lat.precipitation.acidity)
                );
                println!(
                    "      pH Efetivo da Precipitação:{}",
                    match lat.precipitation.ph {
                        Some(ph_val) => format!(" {:.2}", ph_val),
                        None => " N/A (Solvente Não-Aquoso ou Ácido Puro)".to_string(),
                    }
                );
                println!(
                    "      Condensação Superficial:  {}",
                    format_surface_condensation(lat.precipitation.surface_condensation)
                );
            }

            let report_path = format!(
                "precipitation_diagnostics_{}.txt",
                report.planet_id
            );
            let report_json = serde_json::to_string_pretty(&report)?;
            std::fs::write(&report_path, report_json)?;
            println!(
                "\n7. SÍNTESE DO DIAGNÓSTICO exportada para: {}\n",
                report_path
            );
        }
    }

    Ok(())
}