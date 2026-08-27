use astronomicon_app::climate::{
    resolve_advective_surface_temperature, resolve_global_mean_temperature,
};
use astronomicon_app::habitability::{
    resolve_biochemical_viability_at_latitude, resolve_habitability_assessment,
    resolve_photosynthetic_productivity_at_latitude,
    resolve_standard_primary_habitability_at_latitude, BiochemicalViabilityDiagnostic,
    GlobalBiochemicalViabilityDiagnostic, HabitabilityDiagnostic, SurfaceProductivityDiagnostic,
};
use astronomicon_app::hierarchy::find_parent_star;
use astronomicon_core::domain::Planet;
use astronomicon_core::math::gravity::{gravitational_parameter, surface_gravity};
use astronomicon_core::math::habitability::{
    ChemicalToleranceConfidence, ChemosyntheticPathways, EarthSimilarityIndex,
    FirstOrderNutrientLimitation, PlanetaryHabitabilityClassification,
    PrimaryProductivityConfidence, SubsurfaceHabitabilityTier, SurfaceHabitabilityTier,
};
use astronomicon_core::math::radiometry::{escape_velocity, PhotosyntheticFluxSummary};
use astronomicon_core::units::{Angle, Duration, Length};
use astronomicon_db::repositories::{
    atmosphere_repository, hydrosphere_repository, planet_repository, universe_state_repository,
};
use astronomicon_db::save::initialize_save;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComprehensiveHabitabilityReport {
    pub planet_id: Uuid,
    pub planet_name: String,
    pub planet_kind: String,
    pub star_name: String,
    pub star_kind: String,
    pub mass_kg: f64,
    pub equatorial_radius_km: Option<f64>,
    pub surface_gravity_ms2: f64,
    pub escape_velocity_km_s: f64,
    pub surface_pressure_bar: Option<f64>,
    pub global_mean_temperature_k: f64,
    pub global_mean_temperature_c: f64,
    pub earth_similarity: Option<EarthSimilarityIndex>,
    pub classification: PlanetaryHabitabilityClassification,
    pub global_surface_productivity: SurfaceProductivityDiagnostic,
    pub chemosynthetic_pathways: ChemosyntheticPathways,
    pub global_biochemical_viability: GlobalBiochemicalViabilityDiagnostic,
    pub nutrient_limitation: FirstOrderNutrientLimitation,
    pub latitudinal_transect: Vec<LatitudinalHabitabilityDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatitudinalHabitabilityDiagnostic {
    pub latitude_deg: f64,
    pub surface_temperature_k: f64,
    pub surface_temperature_c: f64,
    pub biochemical_viability: BiochemicalViabilityDiagnostic,
    pub photosynthetic_flux: PhotosyntheticFluxSummary,
    pub primary_habitability: astronomicon_core::math::habitability::StandardPrimaryHabitability,
    pub is_liquid_solvent: bool,
}

fn format_surface_tier(tier: SurfaceHabitabilityTier) -> &'static str {
    match tier {
        SurfaceHabitabilityTier::HyperHabitable => {
            "Hiper-Habitável (Condições Superiores às Terrestres)"
        }
        SurfaceHabitabilityTier::HabitableMesophilic => {
            "Habitável Mesofílico (Biologia Complexa / Eucariótica Plena)"
        }
        SurfaceHabitabilityTier::MarginallyHabitableExtremophilic => {
            "Marginalmente Habitável (Dominado por Extremófilos)"
        }
        SurfaceHabitabilityTier::TransientOrPrebiotic => {
            "Transitório / Pré-biótico (Potencial Químico Marginal)"
        }
        SurfaceHabitabilityTier::InhabitableSurface => "Inabitável na Superfície",
    }
}

fn format_subsurface_tier(tier: SubsurfaceHabitabilityTier) -> &'static str {
    match tier {
        SubsurfaceHabitabilityTier::ActiveOceanWorld => {
            "Mundo Oceânico Ativo (Quimiossíntese Hidrotermal Vigorosa)"
        }
        SubsurfaceHabitabilityTier::DormantOceanWorld => {
            "Mundo Oceânico Dormente (Baixa Energia Quimiossintética)"
        }
        SubsurfaceHabitabilityTier::NoSubsurfaceHabitat => "Sem Habitat Subsuperficial",
    }
}

fn format_chemical_confidence(conf: ChemicalToleranceConfidence) -> &'static str {
    match conf {
        ChemicalToleranceConfidence::HighKnownAqueousBiochemistry => {
            "Alta Confiança (Bioquímica Aquosa Terrestre)"
        }
        ChemicalToleranceConfidence::LowSpeculativeCryogenicHydrocarbon => {
            "Baixa Confiança (Bioquímica Especulativa de Hidrocarbonetos Criogênicos)"
        }
        ChemicalToleranceConfidence::InviableOrUnknownBiochemistry => {
            "Inviável / Bioquímica Desconhecida"
        }
    }
}

fn format_primary_confidence(conf: PrimaryProductivityConfidence) -> &'static str {
    match conf {
        PrimaryProductivityConfidence::HighAqueousBiochemistry => {
            "Alta Confiança (Bioquímica Aquosa - Modelo de Miami)"
        }
        PrimaryProductivityConfidence::LowNonAqueousSpeculative => {
            "Baixa Confiança (Solvente Não-Aquoso Especulativo)"
        }
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
        let planet = Planet::try_from(planet_row)?;

        let eq_radius = planet
            .equatorial_radius()
            .unwrap_or_else(|| Length::new(6371e3));
        let mu = gravitational_parameter(planet.mass());
        let g = surface_gravity(mu, eq_radius);
        let v_esc = escape_velocity(mu, eq_radius);

        let star_res = find_parent_star(&pool, planet.orbital_parent()).await;
        let (star_name, star_kind) = match star_res {
            Ok(ref s) => (s.name().to_string(), format!("{:?}", s.kind())),
            Err(_) => ("Nenhuma / Órfão".to_string(), "Desconhecido".to_string()),
        };

        let atm_opt = atmosphere_repository::get_by_planet_id(&pool, &planet_id).await?;
        let hydro_opt = hydrosphere_repository::get_by_planet_id(&pool, &planet_id).await?;

        let global_temp =
            resolve_global_mean_temperature(&pool, planet_id, universe_epoch, at_epoch).await?;

        let assessment: HabitabilityDiagnostic =
            resolve_habitability_assessment(&pool, planet_id, universe_epoch, at_epoch).await?;

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
            let bio_viab = resolve_biochemical_viability_at_latitude(
                &pool,
                planet_id,
                lat_rad,
                universe_epoch,
                at_epoch,
            )
            .await?;
            let photo_flux = resolve_photosynthetic_productivity_at_latitude(
                &pool,
                planet_id,
                lat_rad,
                universe_epoch,
                at_epoch,
            )
            .await?;
            let prim_hab = resolve_standard_primary_habitability_at_latitude(
                &pool,
                planet_id,
                lat_rad,
                universe_epoch,
                at_epoch,
            )
            .await?;

            latitudinal_transect.push(LatitudinalHabitabilityDiagnostic {
                latitude_deg: lat_deg,
                surface_temperature_k: surf_t.value(),
                surface_temperature_c: surf_t.value() - 273.15,
                biochemical_viability: bio_viab,
                photosynthetic_flux: photo_flux,
                primary_habitability: prim_hab,
                is_liquid_solvent: bio_viab.is_liquid_solvent,
            });
        }

        let report = ComprehensiveHabitabilityReport {
            planet_id,
            planet_name: planet.name().to_string(),
            planet_kind: format!("{:?}", planet.kind()),
            star_name,
            star_kind,
            mass_kg: planet.mass().value(),
            equatorial_radius_km: planet.equatorial_radius().map(|r| r.value() / 1000.0),
            surface_gravity_ms2: g.value(),
            escape_velocity_km_s: v_esc.value() / 1000.0,
            surface_pressure_bar: atm_opt
                .as_ref()
                .map(|a| a.surface_pressure().value() / 100_000.0),
            global_mean_temperature_k: global_temp.value(),
            global_mean_temperature_c: global_temp.value() - 273.15,
            earth_similarity: assessment.earth_similarity,
            classification: assessment.classification.clone(),
            global_surface_productivity: assessment.surface_productivity,
            chemosynthetic_pathways: assessment.chemosynthetic_pathways,
            global_biochemical_viability: assessment.biochemical_viability,
            nutrient_limitation: assessment.nutrient_limitation.clone(),
            latitudinal_transect,
        };

        println!(
            "================================================================================"
        );
        println!(
            "RELATÓRIO DE HABITABILIDADE, BIOENERGÉTICA E BIOSFERA: {}",
            report.planet_name.to_uppercase()
        );
        println!(
            "================================================================================"
        );

        println!("1. PARÂMETROS PLANETÁRIOS E CONTEXTO ASTROFÍSICO:");
        println!("  Classificação Planetária:       {}", report.planet_kind);
        println!(
            "  Estrela Hospedeira:            {} ({})",
            report.star_name, report.star_kind
        );
        println!("  Massa Planetária:              {:.3e} kg", report.mass_kg);
        println!(
            "  Raio Equatorial:               {}",
            match report.equatorial_radius_km {
                Some(r) => format!("{:.2} km", r),
                None => "Não Definido".to_string(),
            }
        );
        println!(
            "  Gravidade Superficial:         {:.2} m/s²",
            report.surface_gravity_ms2
        );
        println!(
            "  Velocidade de Escape:          {:.2} km/s",
            report.escape_velocity_km_s
        );
        println!(
            "  Pressão Superficial:           {}",
            match report.surface_pressure_bar {
                Some(p) => format!("{:.4} bar", p),
                None => "Ausente / Vácuo".to_string(),
            }
        );
        println!(
            "  Temperatura Média Global:      {:.2} K ({:.2} °C)",
            report.global_mean_temperature_k, report.global_mean_temperature_c
        );
        println!(
            "  Hidrosfera Superficial:        {}",
            match hydro_opt {
                Some(ref h) => format!(
                    "Presente ({:.1}% de cobertura, {:.1} m de prof. média)",
                    h.surface_coverage_fraction() * 100.0,
                    h.average_depth().value()
                ),
                None => "Ausente / Não Detectada".to_string(),
            }
        );

        println!("\n2. ÍNDICE DE SIMILARIDADE COM A TERRA (EARTH SIMILARITY INDEX - ESI):");
        if let Some(esi) = report.earth_similarity {
            println!(
                "  ESI Global:                    {:.3} ({:.1}%)",
                esi.global,
                esi.global * 100.0
            );
            println!(
                "  ESI Interior:                  {:.3} (Raio: {:.3}, Densidade: {:.3})",
                esi.interior, esi.radius_component, esi.density_component
            );
            println!(
                "  ESI Superfície:                {:.3} (V_esc: {:.3}, Temp: {:.3})",
                esi.surface, esi.escape_velocity_component, esi.surface_temperature_component
            );
        } else {
            println!("  ESI:                           Não Aplicável (Sem Raio Definido)");
        }

        println!("\n3. CLASSIFICAÇÃO GERAL E TIERS DE HABITABILIDADE:");
        println!(
            "  Tier de Superfície:            {}",
            format_surface_tier(report.classification.surface_tier)
        );
        println!(
            "  Tier de Subsuperfície:         {}",
            format_subsurface_tier(report.classification.subsurface_tier)
        );
        println!(
            "  Score de Viabilidade Superf.:  {:.3} ({:.1}%)",
            report.classification.surface_viability_score,
            report.classification.surface_viability_score * 100.0
        );
        println!(
            "  Score de Viabilidade Subsup.:  {:.3} ({:.1}%)",
            report.classification.subsurface_viability_score,
            report.classification.subsurface_viability_score * 100.0
        );
        println!(
            "  Índice Produtividade Fotos.:   {:.3} (SPH Index)",
            report.classification.photosynthetic_productivity_index
        );
        println!(
            "  Potência Quimiossintética:     {:.3e} W",
            report.classification.chemosynthetic_power.value()
        );
        println!(
            "  Fator Nutricional de Redfield: {:.3} ({:.1}%)",
            report.classification.nutrient_availability_factor,
            report.classification.nutrient_availability_factor * 100.0
        );

        println!("\n4. BIOENERGÉTICA FOTOTRÓFICA E RADIAÇÃO FOTOSSINTETICAMENTE ATIVA (PAR):");
        let photo = &report
            .global_surface_productivity
            .theoretical_photosynthetic_flux;
        println!(
            "  Irradiância PAR no Topo (TOA): {:.2} W/m²",
            photo.toa_par_irradiance.value()
        );
        println!(
            "  Irradiância PAR na Superfície: {:.2} W/m²",
            photo.surface_par_irradiance.value()
        );
        println!(
            "  Fração PAR do Espectro Solar:  {:.2}%",
            photo.par_fraction_of_total * 100.0
        );
        println!(
            "  Transmitância Atmosférica PAR: {:.2}%",
            photo.atmospheric_par_transmittance * 100.0
        );
        println!(
            "  Fluxo Teórico Máx. de Biomassa:{:.3e} W/m² (Limite Quântico ~11%)",
            photo.max_biomass_energy_flux.value()
        );

        println!("\n5. PRODUTIVIDADE PRIMÁRIA PADRÃO (SPH / MODELO DE MIAMI):");
        let sph = &report
            .global_surface_productivity
            .empirical_primary_habitability;
        println!(
            "  Índice SPH Normalizado:        {:.3} ({:.1}%)",
            sph.sph_index,
            sph.sph_index * 100.0
        );
        println!(
            "  NPP Efetivo Integrado:         {:.1} g/m²/ano",
            sph.npp_final
        );
        println!(
            "  NPP Limitado por Temperatura:  {:.1} g/m²/ano",
            sph.npp_temperature
        );
        println!(
            "  NPP Limitado por Precipitação: {:.1} g/m²/ano",
            sph.npp_precipitation
        );
        println!(
            "  Confiança Ecológica:           {}",
            format_primary_confidence(sph.confidence)
        );

        println!("\n6. BIOENERGÉTICA QUIMIOSSINTÉTICA E VIAS QUIMIOLITOTRÓFICAS:");
        let chemo = &report.chemosynthetic_pathways;
        println!(
            "  Potência Total Quimiossint.:   {:.3e} W",
            chemo.total_chemical_power.value()
        );
        println!(
            "  Produção Líquida de Biomassa:  {:.3e} W (Eficiência: {:.1}%)",
            chemo.net_biomass_production_power.value(),
            chemo.conversion_efficiency * 100.0
        );
        println!(
            "  - Oxidação de Sulfeto (H2S):   {:.3e} W",
            chemo.h2s_oxidation_power.value()
        );
        println!(
            "  - Metanogênese (CO2 + 4H2):    {:.3e} W",
            chemo.methanogenesis_power.value()
        );
        println!(
            "  - Metanotrofia (CH4 + 2O2):    {:.3e} W",
            chemo.methanotrophy_power.value()
        );
        println!(
            "  - Oxidação de Ferro (Fe2+):    {:.3e} W",
            chemo.iron_oxidation_power.value()
        );

        println!("\n7. ESTEQUIOMETRIA E LIMITAÇÃO DE NUTRIENTES (RAZÃO DE REDFIELD):");
        let nutr = &report.nutrient_limitation;
        println!("  Elemento Mais Limitante:       {}", nutr.limiting_element);
        println!(
            "  Fator de Disponibilidade:      {:.3} ({:.1}%)",
            nutr.availability_factor,
            nutr.availability_factor * 100.0
        );
        println!(
            "  Razões Normalizadas a Redfield: C={:.3} | N={:.3} | P={:.3}",
            nutr.c_ratio_to_redfield, nutr.n_ratio_to_redfield, nutr.p_ratio_to_redfield
        );
        println!(
            "  Gargalo Nutricional Ativo:     {}",
            if nutr.is_phosphorus_limited {
                "Fósforo (P) - Limitação por Intemperismo Crustal"
            } else if nutr.is_nitrogen_limited {
                "Nitrogênio (N) - Limitação Atmosférica / Fixação"
            } else if nutr.is_carbon_limited {
                "Carbono (C) - Escassez de Carbono Inorgânico"
            } else {
                "Equilibrado"
            }
        );

        println!("\n8. RADIOBIOLOGIA, FOTOBIOLOGIA UV E BLINDAGEM DO DNA:");
        let rad = &report
            .global_biochemical_viability
            .average_radiation_tolerance;
        let uv = &report.global_biochemical_viability.average_uv_photobiology;
        println!(
            "  Dose Anual de Radiação Média:  {:.4e} Sv/ano",
            rad.annual_dose.value()
        );
        println!(
            "  Sobrevivência Eucarioto/Complex:{:.2}% | Viável: {}",
            rad.complex_life_survival_fraction * 100.0,
            if rad.is_complex_life_viable {
                "SIM"
            } else {
                "NÃO"
            }
        );
        println!(
            "  Sobrevivência Extremófilo:     {:.2}% | Viável: {}",
            rad.extremophile_survival_fraction * 100.0,
            if rad.is_extremophile_viable {
                "SIM"
            } else {
                "NÃO"
            }
        );
        println!(
            "  Eficiência Blindagem UV do DNA:{:.2}%",
            uv.dna_shielding_efficiency * 100.0
        );
        println!(
            "  UV Efetivo no Topo / Solo:     {:.3e} W/m² / {:.3e} W/m²",
            uv.toa_effective_uv_irradiance.value(),
            uv.surface_effective_uv_irradiance.value()
        );
        println!(
            "  Transmitância Espectral UV:    UVC={:.2e} | UVB={:.2e} | UVA={:.2e}",
            uv.uvc_transmittance, uv.uvb_transmittance, uv.uva_transmittance
        );

        println!("\n9. TOLERÂNCIA FÍSICO-QUÍMICA E SOLVENTE LÍQUIDO SUPERFICIAL:");
        let chem = &report
            .global_biochemical_viability
            .average_chemical_tolerance;
        println!(
            "  Viabilidade Térmica Média:     {:.1}%",
            chem.temperature_viability * 100.0
        );
        println!(
            "  Viabilidade de pH Média:       {}",
            match chem.ph_viability {
                Some(v) => format!("{:.1}%", v * 100.0),
                None => "N/A (Solvente Não-Aquoso)".to_string(),
            }
        );
        println!(
            "  Viabilidade Química Global:    {:.1}%",
            chem.overall_viability * 100.0
        );
        println!(
            "  Fração de Superfície Líquida:  {:.1}% (Área com Solvente Líquido Estável)",
            report
                .global_biochemical_viability
                .liquid_solvent_surface_fraction
                * 100.0
        );
        println!(
            "  Confiança Geoquímica:          {}",
            format_chemical_confidence(chem.confidence)
        );

        println!("\n10. TRANSECTO LATITUDINAL DE HABITABILIDADE (Equador -> Polo):");
        println!(
            "-------------------------------------------------------------------------------------------------------------------------------"
        );
        println!(
            "{:<6} | {:<7} | {:<12} | {:<12} | {:<12} | {:<10} | {:<10} | {:<10} | {:<8}",
            "Lat",
            "T_surf",
            "Viab.Compos.",
            "PAR_Surf(W)",
            "NPP_Final",
            "SPH Index",
            "Blind.UV",
            "Sobrev.Euc",
            "Líquido?"
        );
        println!(
            "-------------------------------------------------------------------------------------------------------------------------------"
        );
        for lat in &report.latitudinal_transect {
            println!(
                "{:>4.0}° | {:>5.1} K | {:>11.1}% | {:>11.2} | {:>10.1} | {:>9.3} | {:>9.1}% | {:>9.1}% | {:<8}",
                lat.latitude_deg,
                lat.surface_temperature_k,
                lat.biochemical_viability.composite_viability_score * 100.0,
                lat.photosynthetic_flux.surface_par_irradiance.value(),
                lat.primary_habitability.npp_final,
                lat.primary_habitability.sph_index,
                lat.biochemical_viability
                    .uv_photobiology
                    .dna_shielding_efficiency
                    * 100.0,
                lat.biochemical_viability
                    .radiation_tolerance
                    .complex_life_survival_fraction
                    * 100.0,
                if lat.is_liquid_solvent { "SIM" } else { "NÃO" }
            );
        }
        println!(
            "-------------------------------------------------------------------------------------------------------------------------------"
        );

        println!("\n11. DETALHAMENTO DA CADEIA POR LATITUDE:");
        for lat in &report.latitudinal_transect {
            println!(
                "\n  === LATITUDE {:>4.1}° (T_surf: {:.2} K / {:.2} °C | Solvente Líquido: {}) ===",
                lat.latitude_deg,
                lat.surface_temperature_k,
                lat.surface_temperature_c,
                if lat.is_liquid_solvent {
                    "SIM"
                } else {
                    "NÃO"
                }
            );
            println!(
                "    Bioenergética & PAR: TOA={:.1} W/m² -> Solo={:.1} W/m² (Transm: {:.1}%) | Max Biomassa={:.3e} W/m²",
                lat.photosynthetic_flux.toa_par_irradiance.value(),
                lat.photosynthetic_flux.surface_par_irradiance.value(),
                lat.photosynthetic_flux.atmospheric_par_transmittance * 100.0,
                lat.photosynthetic_flux.max_biomass_energy_flux.value()
            );
            println!(
                "    Modelo de Miami / SPH: NPP_T={:.1} | NPP_P={:.1} | NPP_Final={:.1} g/m²/ano | SPH={:.3}",
                lat.primary_habitability.npp_temperature,
                lat.primary_habitability.npp_precipitation,
                lat.primary_habitability.npp_final,
                lat.primary_habitability.sph_index
            );
            println!(
                "    Fotobiologia UV: Blindagem DNA={:.1}% | UVC={:.1e} | UVB={:.1e} | UVA={:.1e}",
                lat.biochemical_viability
                    .uv_photobiology
                    .dna_shielding_efficiency
                    * 100.0,
                lat.biochemical_viability.uv_photobiology.uvc_transmittance,
                lat.biochemical_viability.uv_photobiology.uvb_transmittance,
                lat.biochemical_viability.uv_photobiology.uva_transmittance
            );
            println!(
                "    Tolerância Química: Térmica={:.1}% | pH={:.1}% | Overall={:.1}%",
                lat.biochemical_viability
                    .chemical_tolerance
                    .temperature_viability
                    * 100.0,
                lat.biochemical_viability
                    .chemical_tolerance
                    .ph_viability
                    .unwrap_or(0.0)
                    * 100.0,
                lat.biochemical_viability
                    .chemical_tolerance
                    .overall_viability
                    * 100.0
            );
            println!(
                "    Radiobiologia: Dose={:.2e} Sv/ano | Sobrevivência Eucarioto={:.1}% | Extremófilo={:.1}%",
                lat.biochemical_viability
                    .radiation_tolerance
                    .annual_dose
                    .value(),
                lat.biochemical_viability
                    .radiation_tolerance
                    .complex_life_survival_fraction
                    * 100.0,
                lat.biochemical_viability
                    .radiation_tolerance
                    .extremophile_survival_fraction
                    * 100.0
            );
            println!(
                "    Score de Viabilidade Composta: {:.1}%",
                lat.biochemical_viability.composite_viability_score * 100.0
            );
        }

        let report_path = format!("habitability_diagnostics_{}.txt", report.planet_id);
        let report_json = serde_json::to_string_pretty(&report)?;
        std::fs::write(&report_path, report_json)?;
        println!(
            "\n12. DIAGNÓSTICO DE HABITABILIDADE EXPORTADO COM SUCESSO PARA: {}\n",
            report_path
        );
    }

    Ok(())
}