use astronomicon_app::climate::resolve_global_mean_temperature;
use astronomicon_app::geology::resolve_planetary_geology;
use astronomicon_app::volcanism::resolve_planetary_volcanism;
use astronomicon_core::domain::Planet;
use astronomicon_core::units::Duration;
use astronomicon_db::repositories::{planet_repository, universe_state_repository};
use astronomicon_db::save::initialize_save;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_url = "sqlite://database/astronomicon.db";
    let pool = initialize_save(db_url).await?;

    let universe_epoch = match universe_state_repository::get(&pool).await? {
        Some(state) => Duration::new(state.seconds_since_j2000_epoch),
        None => Duration::new(0.0),
    };
    let at_epoch = Duration::new(0.0);

    let planet_rows = planet_repository::list_all(&pool).await?;
    if planet_rows.is_empty() {
        println!("Nenhum planeta cadastrado no banco de dados.");
        return Ok(());
    }

    println!("================================================================================");
    println!("                      RELATÓRIO COMPLETO DE VULCANISMO                         ");
    println!("================================================================================");
    println!("Época do Universo : {:.2} anos desde J2000", universe_epoch.value() / 31557600.0);
    println!("Total de Planetas : {}", planet_rows.len());
    println!("================================================================================\n");

    for row in planet_rows {
        let planet = match Planet::try_from(row) {
            Ok(p) => p,
            Err(err) => {
                eprintln!("Erro ao carregar dados do planeta: {}", err);
                continue;
            }
        };

        let planet_id = planet.id();
        let volcanism_diag = match resolve_planetary_volcanism(&pool, planet_id, universe_epoch, at_epoch).await {
            Ok(diag) => diag,
            Err(err) => {
                println!("--------------------------------------------------------------------------------");
                println!("Planeta: {} [{:?}]", planet.name(), planet.kind());
                println!("Erro ao calcular diagnóstico vulcânico: {}", err);
                println!("--------------------------------------------------------------------------------\n");
                continue;
            }
        };

        let geology_diag = resolve_planetary_geology(&pool, planet_id, universe_epoch, at_epoch).await.ok();
        let surface_temp = resolve_global_mean_temperature(&pool, planet_id, universe_epoch, at_epoch).await.ok();

        let radius_km = planet.equatorial_radius().map(|r| r.value() / 1000.0).unwrap_or(0.0);
        let mass_kg = planet.mass().value();
        let kg_per_year = volcanism_diag.global_magma_production_rate.value() * 31557600.0;
        let km3_per_year = if kg_per_year > 0.0 { kg_per_year / (2800.0 * 1e9) } else { 0.0 };

        println!("--------------------------------------------------------------------------------");
        println!("PLANETA: {} (ID: {})", planet.name(), planet_id);
        println!("Tipo: {:?} | Massa: {:.3e} kg | Raio Eq.: {:.1} km", planet.kind(), mass_kg, radius_km);
        if let Some(temp) = surface_temp {
            println!("Temperatura Média Global Superficial: {:.2} K ({:.2} °C)", temp.value(), temp.value() - 273.15);
        }
        if let Some(geo) = geology_diag {
            println!("Regime Tectônico: {:?} | Espessura Litosfera: {:.2} km", geo.tectonic_regime, geo.lithosphere_thickness.value() / 1000.0);
        }
        println!();
        println!("  PROPRIEDADES MAGMÁTICAS / EXTRUSÃO:");
        println!("    Taxa de Produção Global de Magma : {:.3e} kg/s", volcanism_diag.global_magma_production_rate.value());
        println!("    Produção Anual Estimada          : {:.3e} kg/ano (~{:.4} km³/ano)", kg_per_year, km3_per_year);
        println!("    Temperatura do Magma             : {:.2} K ({:.2} °C)", volcanism_diag.magma_temperature.value(), volcanism_diag.magma_temperature.value() - 273.15);
        println!("    Viscosidade Dinâmica do Magma    : {:.3e} Pa·s", volcanism_diag.magma_viscosity.value());
        println!("    Fracionamento Eruptivo           : {:.1}% Efusivo / {:.1}% Explosivo", volcanism_diag.effusive_fraction * 100.0, volcanism_diag.explosive_fraction * 100.0);
        println!("    Oceano de Magma Ativo            : {}", if volcanism_diag.is_magma_ocean { "SIM" } else { "NÃO" });
        println!("    Comportamento Criovulcânico      : {}", if volcanism_diag.is_cryovolcanic { "SIM" } else { "NÃO" });
        println!();
        println!("  TAXAS DE DESGASEIFICAÇÃO VOLCÂNICA (OUTGASSING):");
        println!("    H2O (Vapor d'água)               : {:.3e} kg/s ({:.3e} kg/ano)", volcanism_diag.outgassing_rate_h2o.value(), volcanism_diag.outgassing_rate_h2o.value() * 31557600.0);
        println!("    CO2 (Dióxido de Carbono)         : {:.3e} kg/s ({:.3e} kg/ano)", volcanism_diag.outgassing_rate_co2.value(), volcanism_diag.outgassing_rate_co2.value() * 31557600.0);
        println!("    Enxofre (SO2 + H2S)              : {:.3e} kg/s ({:.3e} kg/ano)", volcanism_diag.outgassing_rate_sulfur.value(), volcanism_diag.outgassing_rate_sulfur.value() * 31557600.0);
        let total_outgassing = volcanism_diag.outgassing_rate_h2o + volcanism_diag.outgassing_rate_co2 + volcanism_diag.outgassing_rate_sulfur;
        println!("    Desgaseificação Total            : {:.3e} kg/s ({:.3e} kg/ano)", total_outgassing.value(), total_outgassing.value() * 31557600.0);
        println!("--------------------------------------------------------------------------------\n");
    }

    println!("================================================================================");
    println!("                           FIM DO RELATÓRIO                                    ");
    println!("================================================================================");

    Ok(())
}