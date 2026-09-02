use astronomicon_app::climate::{
    AtmosphericStratificationDiagnostic,
    CloudCoverDiagnostic,
    CloudLayerDiagnostic,
    ConvectiveInstabilityDiagnostic,
    PlanetaryCirculationDiagnostic,
    SevereWeatherDiagnostic,
    TropopauseDiagnostic,
    WindProfileDiagnostic,
    resolve_atmospheric_stratification,
    resolve_cloud_cover,
    resolve_condensable_species,
    resolve_convective_instability,
    resolve_global_mean_temperature,
    resolve_planetary_circulation,
    resolve_severe_weather,
    resolve_tropopause,
    resolve_wind_profile_at_latitude,
};
use astronomicon_core::domain::Planet;
use astronomicon_core::math::clouds::{
    AtmosphericStability,
    CloudMorphology,
    GlaciationState,
    LightningPotential,
    StormMode,
};
use astronomicon_core::math::gravity::{ gravitational_parameter, surface_gravity };
use astronomicon_core::units::{ Angle, Duration, Length };
use astronomicon_db::repositories::{
    atmosphere_repository,
    hydrosphere_repository,
    planet_repository,
    universe_state_repository,
};
use astronomicon_db::save::initialize_save;
use serde::{ Deserialize, Serialize };
use std::f64::consts::PI;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComprehensiveMeteorologicalDiagnostic {
    pub planet_id: Uuid,
    pub planet_name: String,
    pub global_mean_temperature_k: f64,
    pub surface_pressure_pa: f64,
    pub surface_gravity_ms2: f64,
    pub tropopause: TropopauseDiagnostic,
    pub stratification: AtmosphericStratificationDiagnostic,
    pub convective_instability: ConvectiveInstabilityDiagnostic,
    pub cloud_cover: CloudCoverDiagnostic,
    pub circulation: PlanetaryCirculationDiagnostic,
    pub wind_equator: WindProfileDiagnostic,
    pub wind_midlatitude: WindProfileDiagnostic,
    pub severe_weather: SevereWeatherDiagnostic,
    pub condensable_solvent: SolventDiagnostic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolventDiagnostic {
    pub enthalpy_of_vaporization_j_per_mol: f64,
    pub molar_mass_g_per_mol: f64,
    pub normal_boiling_point_k: f64,
    pub normal_melting_point_k: f64,
    pub surface_humidity_fraction: f64,
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
        GlaciationState::MixedPhase => "Fase Mista (Água + Gelo)",
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

fn print_cloud_layer(name: &str, layer: &CloudLayerDiagnostic) {
    println!("  [{}]", name);
    println!(
        "    Altitude da Camada:       {:.2} km -> {:.2} km (Ponto Médio: {:.2} km)",
        layer.base_altitude.value() / 1000.0,
        layer.top_altitude.value() / 1000.0,
        layer.representative_altitude.value() / 1000.0
    );
    println!("    Cobertura Fracionária:    {:.2}%", layer.cloud_fraction * 100.0);
    println!(
        "    Umidade Relativa (RH):    {:.2}% (RH Crítica: {:.2}%)",
        layer.relative_humidity * 100.0,
        layer.critical_relative_humidity * 100.0
    );
    println!(
        "    Conteúdo Líquido (LWC):   {:.4e} kg/m³ ({:.2} g/m³)",
        layer.liquid_water_content.value(),
        layer.liquid_water_content.value() * 1000.0
    );
    println!(
        "    Conteúdo de Gelo (IWC):   {:.4e} kg/m³ ({:.2} g/m³)",
        layer.ice_water_content.value(),
        layer.ice_water_content.value() * 1000.0
    );
    println!(
        "    Fração de Gelo / Fase:    {:.1}% | {}",
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
            let eq_radius = planet.equatorial_radius().unwrap_or_else(|| Length::new(6371e3));
            let mu = gravitational_parameter(planet.mass());
            let g = surface_gravity(mu, eq_radius);

            let global_temp = resolve_global_mean_temperature(
                &pool,
                planet_id,
                universe_epoch,
                at_epoch
            ).await?;
            let tropo = resolve_tropopause(&pool, planet_id, universe_epoch, at_epoch).await?;
            let strat = resolve_atmospheric_stratification(
                &pool,
                planet_id,
                universe_epoch,
                at_epoch
            ).await?;
            let convective = resolve_convective_instability(
                &pool,
                planet_id,
                universe_epoch,
                at_epoch
            ).await?;
            let clouds = resolve_cloud_cover(&pool, planet_id, universe_epoch, at_epoch).await?;
            let severe = resolve_severe_weather(&pool, planet_id, universe_epoch, at_epoch).await?;
            let circulation = resolve_planetary_circulation(
                &pool,
                planet_id,
                universe_epoch,
                at_epoch
            ).await?;

            let wind_eq = resolve_wind_profile_at_latitude(
                &pool,
                planet_id,
                Angle::new(0.0),
                universe_epoch,
                at_epoch
            ).await?;
            let lat_mid = Angle::new((45.0 * PI) / 180.0);
            let wind_mid = resolve_wind_profile_at_latitude(
                &pool,
                planet_id,
                lat_mid,
                universe_epoch,
                at_epoch
            ).await?;

            let (solvent_props, solvent_mm, hum) = resolve_condensable_species(
                &pool,
                planet_id
            ).await?;
            let hydro_opt = hydrosphere_repository::get_by_planet_id(&pool, &planet_id).await?;

            let diagnostic = ComprehensiveMeteorologicalDiagnostic {
                planet_id,
                planet_name: planet.name().to_string(),
                global_mean_temperature_k: global_temp.value(),
                surface_pressure_pa: atm.surface_pressure().value(),
                surface_gravity_ms2: g.value(),
                tropopause: tropo,
                stratification: strat,
                convective_instability: convective,
                cloud_cover: clouds,
                circulation,
                wind_equator: wind_eq,
                wind_midlatitude: wind_mid,
                severe_weather: severe,
                condensable_solvent: SolventDiagnostic {
                    enthalpy_of_vaporization_j_per_mol: solvent_props.enthalpy_of_vaporization,
                    molar_mass_g_per_mol: solvent_mm.value() * 1000.0,
                    normal_boiling_point_k: solvent_props.normal_boiling_point.value(),
                    normal_melting_point_k: solvent_props.normal_melting_point.value(),
                    surface_humidity_fraction: hum,
                },
            };

            println!(
                "================================================================================"
            );
            println!("RELATÓRIO METEOROLÓGICO E MODELAGEM DE CONVECÇÃO: {}", planet.name());
            println!(
                "================================================================================"
            );

            println!("1. PROPRIEDADES FUNDAMENTAIS DA ATMOSFERA E SUPERFÍCIE:");
            println!("  Massa Planetária:              {:.3e} kg", planet.mass().value());
            println!("  Raio Equatorial:               {:.2} km", eq_radius.value() / 1000.0);
            println!("  Gravidade Superficial:         {:.2} m/s²", g.value());
            println!(
                "  Pressão Superficial:           {:.4} bar ({:.1} Pa)",
                atm.surface_pressure().value() / 100_000.0,
                atm.surface_pressure().value()
            );
            println!(
                "  Temperatura Média Global:      {:.2} K ({:.2} °C)",
                global_temp.value(),
                global_temp.value() - 273.15
            );
            println!("  Efeito Estufa:                 {:.2} K", atm.greenhouse_effect().value());
            if let Ok(scale_h) = atm.scale_height(g, global_temp) {
                println!("  Altura de Escala (H):          {:.2} km", scale_h.value() / 1000.0);
            }
            if let Ok(mean_mm) = atm.mean_molar_mass() {
                println!("  Massa Molar Média do Ar:       {:.3} g/mol", mean_mm.value() * 1000.0);
            }
            if let Ok(dens) = atm.density_at_surface(global_temp) {
                println!("  Densidade do Ar à Superfície:  {:.4} kg/m³", dens.value());
            }

            println!("\n2. SOLVENTE CONDENSÁVEL E HIDROSFERA:");
            println!("  Massa Molar do Solvente:       {:.3} g/mol", solvent_mm.value() * 1000.0);
            println!(
                "  Entalpia de Vaporização:       {:.1} J/mol",
                solvent_props.enthalpy_of_vaporization
            );
            println!(
                "  Ponto de Ebulição Normal:      {:.2} K ({:.2} °C)",
                solvent_props.normal_boiling_point.value(),
                solvent_props.normal_boiling_point.value() - 273.15
            );
            println!(
                "  Ponto de Fusão Normal:         {:.2} K ({:.2} °C)",
                solvent_props.normal_melting_point.value(),
                solvent_props.normal_melting_point.value() - 273.15
            );
            println!("  Umidade Relativa Superficial:  {:.1}%", hum * 100.0);
            if let Some(hydro) = hydro_opt {
                println!(
                    "  Cobertura Oceânica Líquida:    {:.1}%",
                    hydro.surface_coverage_fraction() * 100.0
                );
                println!("  Profundidade Média do Oceano:  {:.2} m", hydro.average_depth().value());
                println!(
                    "  Salinidade / Fração Soluto:    {:.2}%",
                    hydro.salinity_or_solute_mass_fraction() * 100.0
                );
            } else {
                println!(
                    "  Hidrosfera Superficial:        Não Detectada (Atmosfera Seca/Condensação Pura)"
                );
            }

            println!("\n3. ESTRATIFICAÇÃO TÉRMICA E TROPOPAUSA:");
            println!(
                "  Temperatura de Eq. Radiativo:  {:.2} K ({:.2} °C)",
                tropo.radiative_equilibrium_temperature.value(),
                tropo.radiative_equilibrium_temperature.value() - 273.15
            );
            println!(
                "  Temperatura de Pele (Skin T):  {:.2} K ({:.2} °C)",
                tropo.skin_temperature.value(),
                tropo.skin_temperature.value() - 273.15
            );
            println!(
                "  Altitude da Tropopausa:        {:.2} km",
                tropo.tropopause_altitude.value() / 1000.0
            );
            println!(
                "  Temperatura na Tropopausa:     {:.2} K ({:.2} °C)",
                tropo.tropopause_temperature.value(),
                tropo.tropopause_temperature.value() - 273.15
            );
            println!(
                "  Gradiente Térmico Ambiental:   {:.2} K/km",
                tropo.lapse_rate.value() * 1000.0
            );
            println!(
                "  Ponto de Orvalho Superficial:  {:.2} K ({:.2} °C)",
                strat.surface_dew_point.value(),
                strat.surface_dew_point.value() - 273.15
            );
            println!(
                "  Nível de Condensação (LCL):    {:.2} km",
                strat.lcl_altitude.value() / 1000.0
            );
            println!(
                "  Topo Térmico de Nuvem:         {:.2} km",
                strat.cloud_top_altitude.value() / 1000.0
            );
            println!(
                "  Nível de Congelamento:         {:.2} km",
                clouds.freezing_level.value() / 1000.0
            );

            println!("\n4. TERMODINÂMICA CONVECTIVA E PARCELAS DE AR:");
            println!(
                "  Taxa Adiabática Seca (Γd):     {:.2} K/km",
                convective.dry_adiabatic_lapse_rate.value() * 1000.0
            );
            println!(
                "  Taxa Adiabática Úmida (Γm):    {:.2} K/km",
                convective.moist_adiabatic_lapse_rate.value() * 1000.0
            );
            println!("  Estado de Estabilidade:        {}", format_stability(convective.stability));
            println!(
                "  Morfologia de Nuvens:          {}",
                format_morphology(convective.morphology)
            );
            println!("  Convecção Profunda com Bigorna:{}", if convective.is_deep_convection {
                " Sim (Topo Próximo à Tropopausa)"
            } else {
                " Não (Convecção Rasa / Limitada)"
            });
            println!("  Nível de Livre Convecção (LFC):{}", match convective.lfc_altitude {
                Some(z) => format!(" {:.2} km", z.value() / 1000.0),
                None => " Inexistente (Sem Flutuabilidade Positiva Espontânea)".to_string(),
            });
            println!("  Nível de Equilíbrio (EL):      {}", match convective.equilibrium_level {
                Some(z) => format!(" {:.2} km", z.value() / 1000.0),
                None => " Inexistente".to_string(),
            });
            println!("  CAPE (Energia Convectiva):     {:.1} J/kg", convective.cape.value());
            println!("  CIN (Inibição Convectiva):     {:.1} J/kg", convective.cin.value());

            println!("\n5. MACROFÍSICA E MICROFÍSICA DE NUVENS POR CAMADA:");
            println!(
                "  Cobertura Nebulosa Total:      {:.1}%",
                clouds.total_cloud_fraction * 100.0
            );
            print_cloud_layer("Camada Baixa (0 - 25% da Tropopausa)", &clouds.low_cloud);
            print_cloud_layer("Camada Média (25% - 62.5% da Tropopausa)", &clouds.mid_cloud);
            print_cloud_layer("Camada Alta (62.5% - 100% da Tropopausa)", &clouds.high_cloud);

            println!("\n6. DINÂMICA GLOBAL, CIRCULAÇÃO E CISALHAMENTO DE VENTO:");
            println!(
                "  Velocidade Angular (Ω):        {:.4e} rad/s",
                circulation.angular_velocity.value()
            );
            println!("  Células de Circulação / Hemisf:{}", circulation.circulation_cells);
            println!(
                "  Raio de Deformação de Rossby:  {:.1} km",
                circulation.rossby_deformation_radius.value() / 1000.0
            );
            println!(
                "  Escala de Rhines:              {:.1} km",
                circulation.rhines_scale.value() / 1000.0
            );
            println!(
                "  Eficiência de Redistribuição:  {:.2}%",
                circulation.thermal_redistribution_efficiency * 100.0
            );
            println!(
                "  Vento Superficial Equatorial:  {:.2} m/s ({:.1} km/h)",
                wind_eq.surface_wind_speed.value(),
                wind_eq.surface_wind_speed.value() * 3.6
            );
            println!(
                "  Corrente de Jato (Lat 45°):    {:.2} m/s ({:.1} km/h)",
                wind_mid.jet_stream_speed.value(),
                wind_mid.jet_stream_speed.value() * 3.6
            );
            println!(
                "  Cisalhamento Vertical de Vento:{:.2} m/s ({:.1} km/h)",
                severe.bulk_wind_shear.value(),
                severe.bulk_wind_shear.value() * 3.6
            );

            println!("\n7. METEOROLOGIA SEVERA, ELETRIFICAÇÃO E CICLOGÊNESE:");
            println!("  Bulk Richardson Number (BRN):  {:.2}", severe.bulk_richardson_number);
            println!("  Modo de Tempestade Convectiva: {}", format_storm_mode(severe.storm_mode));
            println!(
                "  Potencial de Relâmpagos:       {}",
                format_lightning(severe.lightning_potential)
            );
            println!(
                "  Intensidade Potencial Ciclones:{:.2} m/s ({:.1} km/h)",
                severe.tropical_cyclone_potential_intensity.value(),
                severe.tropical_cyclone_potential_intensity.value() * 3.6
            );
            println!("  Ciclogênese Tropical Favitada: {}", if severe.is_cyclogenesis_favorable {
                "Sim (Termodinâmica e Força de Coriolis Favitáveis)"
            } else {
                "Não (Ausência de Oceano ou Inércia Térmica/Coriolis Insuficiente)"
            });

            println!("\n8. SERIALIZAÇÃO JSON DO MODELO CLIMATOLÓGICO COMPLETO:");
            println!("{}", serde_json::to_string_pretty(&diagnostic)?);
            println!(
                "================================================================================\n"
            );
        }
    }

    Ok(())
}
