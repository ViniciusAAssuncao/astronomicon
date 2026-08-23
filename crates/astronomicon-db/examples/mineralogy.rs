use astronomicon_app::{
    resolve_planetary_differentiation, resolve_planetary_mineralogy,
};
use astronomicon_core::domain::Planet;
use astronomicon_core::units::Duration;
use astronomicon_db::repositories::{planet_repository, universe_state_repository};
use astronomicon_db::save::initialize_save;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://database/astronomicon.db".to_string());
    let pool = initialize_save(&db_url).await?;

    let universe_state_row = universe_state_repository::get(&pool)
        .await?
        .expect("universe_state not found");
    let universe_epoch = Duration::new(universe_state_row.seconds_since_j2000_epoch);
    let at_epoch = Duration::new(0.0);

    let planet_rows = planet_repository::list_all(&pool).await?;
    let mut planets: Vec<Planet> = planet_rows
        .into_iter()
        .map(Planet::try_from)
        .collect::<Result<Vec<_>, _>>()?;

    let target_order = ["Hadab", "Jatur", "Meros", "Avizina", "Jena"];
    planets.sort_by_key(|p| {
        target_order
            .iter()
            .position(|&name| name == p.name())
            .unwrap_or(usize::MAX)
    });

    println!("{}", "=".repeat(105));
    println!(
        "{:^105}",
        "ASTRONOMICON - RELATÓRIO GEOQUÍMICO, PETROLÓGICO E MINERALÓGICO PLANETÁRIO"
    );
    println!(
        "{:^105}",
        "(Hadab, Jatur, Meros, Avizina e Jena)"
    );
    println!("{}", "=".repeat(105));

    for planet in &planets {
        if !target_order.contains(&planet.name()) {
            continue;
        }

        let planet_id = planet.id();

        let min_diag =
            resolve_planetary_mineralogy(&pool, planet_id, universe_epoch, at_epoch).await?;
        let diff_diag = resolve_planetary_differentiation(&pool, planet_id).await?;

        let abundance = &min_diag.abundance;
        let crustal = &min_diag.crustal_mineralogy;
        let ore = &min_diag.ore_potential;
        let normative = &crustal.normative_mineralogy;

        let disk_t = abundance.disk_temperature.value();
        let refr_pct = abundance.refractory_fraction * 100.0;
        let vol_pct = abundance.volatile_fraction * 100.0;

        let cmf_pct = abundance.core_mass_fraction * 100.0;
        let mmf_pct = abundance.mantle_mass_fraction * 100.0;

        let qtz_pct = normative.quartz * 100.0;
        let plag_pct = normative.plagioclase * 100.0;
        let kspar_pct = normative.k_feldspar * 100.0;
        let pyx_pct = normative.pyroxene * 100.0;
        let ol_pct = normative.olivine * 100.0;
        let felsic_pct = crustal.felsic_fraction * 100.0;
        let mafic_pct = crustal.mafic_fraction * 100.0;

        let petro_class = if normative.is_felsic() {
            "Crosta Félsica / Granítica (Rica em Sílica e Álcalis)"
        } else if normative.is_ultramafic() {
            "Crosta Ultramáfica / Peridotítica (Primitiva Manto-Derivada)"
        } else if normative.is_mafic() {
            "Crosta Máfica / Basáltica (Rica em Piroxênios e Olivina)"
        } else {
            "Crosta Intermediária / Andesítica-Diorítica"
        };

        println!("{}", "-".repeat(105));
        println!(
            "CORPO CELESTE: {:<12} | TIPO: {:<12} | REGIME: {:?}",
            planet.name(),
            format!("{:?}", planet.kind()),
            crustal.tectonic_regime
        );
        println!("{}", "-".repeat(105));

        println!("[1] CONDIÇÕES PRIMORDIAIS E ACREÇÃO NEBULAR:");
        println!(
            "  Temperatura do Disco na Órbita : {:>10.2} K",
            disk_t
        );
        println!(
            "  Fração de Elementos Refratários: {:>10.2} %",
            refr_pct
        );
        println!(
            "  Fração de Voláteis Retidos     : {:>10.2} %",
            vol_pct
        );
        println!(
            "  Razões Molares Nebulares       : Mg/Si = {:<6.3} | Fe/Si = {:<6.3} | C/O = {:<6.3}",
            abundance.mg_si_ratio, abundance.fe_si_ratio, abundance.c_o_ratio
        );
        println!();

        println!("[2] DIFERENCIAÇÃO PLANETÁRIA E PARTIÇÃO NÚCLEO-MANTO:");
        println!(
            "  Fração de Massa do Núcleo (CMF): {:>10.2} %",
            cmf_pct
        );
        println!(
            "  Fração de Massa Mantélica (MMF): {:>10.2} %",
            mmf_pct
        );
        println!(
            "  Composição do Núcleo Metálico  : Fe = {:>5.2} % | Ni = {:>5.2} %",
            diff_diag.core_fe_fraction * 100.0,
            diff_diag.core_ni_fraction * 100.0
        );
        println!(
            "  Geoantropometria Mantélica     : Mg# = {:<6.3} | Mg/Si = {:<6.3} | Fe/Si = {:<6.3}",
            diff_diag.mantle_mg_number,
            diff_diag.mantle_mg_si_ratio,
            diff_diag.mantle_fe_si_ratio
        );
        println!();

        println!("[3] PETROLOGIA CRUSTAL E MINERALOGIA NORMATIVA (CIPW):");
        println!("  Classificação Petrológica      : {}", petro_class);
        println!(
            "  Presença de Água / Hidratação  : {}",
            if crustal.has_water { "SIM (Facilita Magmatismo Diferenciado)" } else { "NÃO (Magmatismo Seco Anidro)" }
        );
        println!(
            "  Índices de Cor e Densidade     : Félsico = {:>5.1} % | Máfico = {:>5.1} %",
            felsic_pct, mafic_pct
        );
        println!("  Assembleia Mineral Normativa   :");
        println!("    - Quartzo (SiO2)             : {:>6.2} %", qtz_pct);
        println!("    - Plagioclásio (Ca/Na Feld)  : {:>6.2} %", plag_pct);
        println!("    - Feldspato Potássico (K-Spar): {:>6.2} %", kspar_pct);
        println!("    - Piroxênios (Di/Hy)         : {:>6.2} %", pyx_pct);
        println!("    - Olivina (Forsterita/Faia)  : {:>6.2} %", ol_pct);

        println!("  Óxidos Dominantes na Crosta    :");
        print!("   ");
        for (i, ox) in crustal.dominant_oxides.iter().take(6).enumerate() {
            print!(" {}: {:>5.2}%", ox.formula, ox.mass_fraction * 100.0);
            if i < 5 {
                print!(" |");
            }
        }
        println!();
        println!();

        println!("[4] POTENCIAL METALOGENÉTICO E DEPÓSITOS MINERAIS:");
        println!(
            "  Sistemas Geológicos Ativos     : Hidrotermal = {:<5} | Evaporítico = {:<5} | BIFs = {:<5}",
            if ore.hydrothermal_active { "SIM" } else { "NÃO" },
            if ore.evaporite_active { "SIM" } else { "NÃO" },
            if ore.bif_active { "SIM" } else { "NÃO" }
        );
        println!(
            "  Potencial Metalogenético (0-1) : Au: {:<4.2} | Cu: {:<4.2} | Fe: {:<4.2} | Li: {:<4.2} | U: {:<4.2}",
            ore.gold_potential,
            ore.copper_potential,
            ore.iron_potential,
            ore.lithium_potential,
            ore.uranium_potential
        );

        if !ore.deposits.is_empty() {
            println!("  Jazidas e Depósitos Minerais Modelados:");
            println!(
                "    {:<32} {:<6} {:<18} {:<10} {:<12} {:<14}",
                "NOME DO DEPÓSITO", "ALVO", "TIPO GENÉTICO", "PROB.", "ENRIQUEC.", "TEOR ESTIMADO"
            );
            for dep in &ore.deposits {
                let grade_str = if dep.estimated_grade_ppm >= 10000.0 {
                    format!("{:.2} %", dep.estimated_grade_ppm / 10000.0)
                } else if dep.estimated_grade_ppm >= 1.0 {
                    format!("{:.1} ppm", dep.estimated_grade_ppm)
                } else {
                    format!("{:.3} ppm", dep.estimated_grade_ppm)
                };

                println!(
                    "    {:<32} {:<6} {:<18} {:>7.1}% {:>10.1}x {:>14}",
                    dep.name,
                    dep.target_element,
                    dep.deposit_type,
                    dep.probability * 100.0,
                    dep.enrichment_factor,
                    grade_str
                );
            }
        } else {
            println!("  Jazidas e Depósitos Minerais Modelados: Nenhum depósito econômico significativo previsto.");
        }

        println!();
    }

    println!("{}", "=".repeat(105));
    println!("Status da Validação: DIAGNÓSTICO MINERALÓGICO E PETROLÓGICO CONCLUÍDO COM SUCESSO.");
    println!("{}", "=".repeat(105));

    Ok(())
}