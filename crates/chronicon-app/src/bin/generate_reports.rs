use astronomicon_core::units::constants::ASTRONOMICAL_UNIT;
use astronomicon_core::units::Duration;
use chronicon_app::definition::resolve_calendar_definition;
use chronicon_app::tick::resolve_calendar_tick;
use chronicon_core::domain::IntercalationAnalysis;
use chronicon_db::connection::{open_pool, DATABASE_URL};
use chronicon_db::repositories::calendar as calendar_repo;
use std::fs;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = open_pool(DATABASE_URL).await?;

    let output_dir = Path::new("crates/chronicon-app/reports");
    fs::create_dir_all(output_dir)?;

    let calendars = calendar_repo::list_all(&pool).await?;

    if calendars.is_empty() {
        println!("Nenhum calendário encontrado no banco de dados.");
        return Ok(());
    }

    let mut index_content = String::from("# Índice de Calendários Resolvidos\n\n");
    index_content.push_str("| Nome do Calendário | Planeta ID | Estrutura | Convenção Dia | Convenção Ano | Arquivo Relatório |\n");
    index_content.push_str("| :--- | :--- | :--- | :--- | :--- | :--- |\n");

    for cal in &calendars {
        let cal_id = cal.id();
        let resolved = resolve_calendar_definition(&pool, &cal_id).await?;

        let tick_epoch = resolve_calendar_tick(&pool, &cal_id, Duration::new(0.0)).await?;
        let tick_one_year = resolve_calendar_tick(&pool, &cal_id, resolved.year_duration).await?;

        let report_md = build_full_markdown_report(&resolved, &tick_epoch, &tick_one_year);

        let sanitized_name = cal
            .name()
            .to_lowercase()
            .replace(|c: char| !c.is_alphanumeric() && c != '_' && c != '-', "_");
        let filename = format!("{}_{}.md", sanitized_name, cal_id);
        let file_path = output_dir.join(&filename);

        fs::write(&file_path, report_md)?;
        println!("Relatório gerado com sucesso: {}", file_path.display());

        index_content.push_str(&format!(
            "| {} | `{}` | `{:?}` | `{:?}` | `{:?}` | [{}]({}) |\n",
            cal.name(),
            cal.planet_id(),
            cal.structure(),
            cal.day_convention(),
            cal.year_convention(),
            filename,
            filename
        ));
    }

    let index_path = output_dir.join("README.md");
    fs::write(&index_path, index_content)?;
    println!("Índice atualizado: {}", index_path.display());

    Ok(())
}

fn build_full_markdown_report(
    resolved: &chronicon_app::definition::ResolvedCalendar,
    tick_epoch: &chronicon_app::tick::CalendarTick,
    tick_one_year: &chronicon_app::tick::CalendarTick,
) -> String {
    let mut md = String::new();
    let def = resolved.definition();
    let skel = resolved.skeleton();

    let local_day_s = resolved.day_duration.value().max(1e-6);
    let local_year_s = resolved.year_duration.value().max(1e-6);

    md.push_str(&format!("# Relatório Técnico de Calendário: {}\n\n", def.name()));

    md.push_str("## 1. Definição Geral e Metadados\n\n");
    md.push_str("| Parâmetro | Valor |\n| :--- | :--- |\n");
    md.push_str(&format!("| **ID do Calendário** | `{}` |\n", def.id()));
    md.push_str(&format!("| **ID do Planeta** | `{}` |\n", def.planet_id()));
    md.push_str(&format!("| **Nome do Planeta** | {} |\n", skel.planet_name));
    md.push_str(&format!("| **Tipo de Estrutura** | `{:?}` |\n", def.structure()));
    md.push_str(&format!("| **Convenção de Dia** | `{:?}` |\n", def.day_convention()));
    md.push_str(&format!("| **Convenção de Ano** | `{:?}` |\n", def.year_convention()));
    md.push_str(&format!("| **Época Inicial** | `{:.4}` s desde J2000 |\n", def.epoch().value()));
    md.push_str(&format!("| **Duração Base do Dia Adotada** | {:.4} s |\n", resolved.day_duration.value()));
    md.push_str(&format!("| **Duração Base do Ano Adotada** | {:.4} s ({:.4} dias locais) |\n", resolved.year_duration.value(), resolved.year_duration.value() / local_day_s));
    md.push_str(&format!("| **Descrição do Evento Fundador** | {} |\n\n", def.founding_event_description().unwrap_or("N/A")));

    md.push_str("## 2. Dinâmica do Dia Planetário e Rotação\n\n");
    md.push_str("| Parâmetro de Rotação | Valor |\n| :--- | :--- |\n");
    md.push_str(&format!("| **Classificação de Rotação** | `{:?}` |\n", skel.day_info.classification()));
    md.push_str(&format!(
        "| **Dia Sideral** | {} |\n",
        skel.day_info
            .sidereal_day()
            .map(|d| format!("{:.4} s ({:.4} dias locais)", d.value(), d.value() / local_day_s))
            .unwrap_or_else(|| "Indisponível".to_string())
    ));
    md.push_str(&format!(
        "| **Dia Solar** | {} |\n",
        skel.day_info
            .solar_day()
            .map(|d| format!("{:.4} s ({:.4} dias locais)", d.value(), d.value() / local_day_s))
            .unwrap_or_else(|| "Indisponível (ex: Bloqueio Síncrono Puro)".to_string())
    ));
    md.push_str(&format!(
        "| **Período Orbital Planetário** | {} |\n\n",
        skel.day_info
            .orbital_period()
            .map(|d| format!("{:.4} s ({:.4} dias locais)", d.value(), d.value() / local_day_s))
            .unwrap_or_else(|| "Indisponível".to_string())
    ));

    md.push_str("## 3. Dinâmica do Ano e Precessão Axial\n\n");
    md.push_str("| Parâmetro Orbital / Precessão | Valor |\n| :--- | :--- |\n");
    md.push_str(&format!(
        "| **Ano Sideral** | {} |\n",
        skel.year_info
            .sidereal_year()
            .map(|d| format!("{:.4} s ({:.4} dias locais)", d.value(), d.value() / local_day_s))
            .unwrap_or_else(|| "Indisponível".to_string())
    ));
    md.push_str(&format!(
        "| **Ano Tropical** | {} |\n",
        skel.year_info
            .tropical_year()
            .map(|d| format!("{:.4} s ({:.4} dias locais)", d.value(), d.value() / local_day_s))
            .unwrap_or_else(|| "Indisponível".to_string())
    ));
    md.push_str(&format!(
        "| **Ano Anomalístico** | {} |\n",
        skel.year_info
            .anomalistic_year()
            .map(|d| format!("{:.4} s ({:.4} dias locais)", d.value(), d.value() / local_day_s))
            .unwrap_or_else(|| "Indisponível".to_string())
    ));
    let prec_rate = skel.year_info.axial_precession_rate().value();
    let prec_arcsec_yr = prec_rate * (180.0 / std::f64::consts::PI) * 3600.0 * local_year_s;
    md.push_str(&format!("| **Taxa de Precessão Axial** | {:.8e} rad/s ({:.4} arcsec/ano local) |\n", prec_rate, prec_arcsec_yr));
    md.push_str(&format!(
        "| **Período de Precessão Axial** | {} |\n",
        skel.year_info
            .axial_precession_period()
            .map(|d| format!("{:.2} anos locais ({:.4e} s)", d.value() / local_year_s, d.value()))
            .unwrap_or_else(|| "Indisponível / Nula".to_string())
    ));
    md.push_str(&format!("| **Dados Físicos de Precessão Disponíveis** | `{}` |\n\n", skel.year_info.has_precession_data()));

    md.push_str("## 4. Estrutura e Duração das Estações do Ano\n\n");
    let sea = &skel.seasonal_structure;
    md.push_str("| Propriedade Sazonal | Valor |\n| :--- | :--- |\n");
    md.push_str(&format!("| **Classificação de Sazonalidade** | `{:?}` |\n", sea.classification()));
    md.push_str(&format!("| **Índice de Precessão Climática** | {:.6} |\n", sea.climatic_precession_index()));
    md.push_str(&format!("| **Período Orbital de Referência** | {:.4} s ({:.4} dias locais) |\n\n", sea.orbital_period().value(), sea.orbital_period().value() / local_day_s));

    if let Some(cp) = sea.cardinal_points() {
        md.push_str("### 4.1 Pontos Cardinais Orbitais (Hemisfério Norte)\n\n");
        md.push_str("| Ponto Cardinal | Anomalia Verdadeira (rad) | Anomalia Verdadeira (°) | Tempo desde Periastro (s) | Tempo desde Periastro (dias locais) |\n| :--- | :--- | :--- | :--- | :--- |\n");
        md.push_str(&format!(
            "| **Equinócio de Primavera** | {:.6} | {:.2}° | {:.2} | {:.2} |\n",
            cp.north_spring_equinox_true_anomaly.value(),
            cp.north_spring_equinox_true_anomaly.value().to_degrees(),
            cp.north_spring_equinox_time.value(),
            cp.north_spring_equinox_time.value() / local_day_s
        ));
        md.push_str(&format!(
            "| **Solstício de Verão** | {:.6} | {:.2}° | {:.2} | {:.2} |\n",
            cp.north_summer_solstice_true_anomaly.value(),
            cp.north_summer_solstice_true_anomaly.value().to_degrees(),
            cp.north_summer_solstice_time.value(),
            cp.north_summer_solstice_time.value() / local_day_s
        ));
        md.push_str(&format!(
            "| **Equinócio de Outono** | {:.6} | {:.2}° | {:.2} | {:.2} |\n",
            cp.north_autumn_equinox_true_anomaly.value(),
            cp.north_autumn_equinox_true_anomaly.value().to_degrees(),
            cp.north_autumn_equinox_time.value(),
            cp.north_autumn_equinox_time.value() / local_day_s
        ));
        md.push_str(&format!(
            "| **Solstício de Inverno** | {:.6} | {:.2}° | {:.2} | {:.2} |\n\n",
            cp.north_winter_solstice_true_anomaly.value(),
            cp.north_winter_solstice_true_anomaly.value().to_degrees(),
            cp.north_winter_solstice_time.value(),
            cp.north_winter_solstice_time.value() / local_day_s
        ));
    }

    if let (Some(nd), Some(sd)) = (sea.northern_hemisphere_durations(), sea.southern_hemisphere_durations()) {
        md.push_str("### 4.2 Duração das Estações por Hemisfério\n\n");
        md.push_str("| Estação | Duração Hemisfério Norte | Duração Hemisfério Sul |\n| :--- | :--- | :--- |\n");
        md.push_str(&format!(
            "| **Primavera** | {:.2} dias locais ({:.2} s) | {:.2} dias locais ({:.2} s) |\n",
            nd.spring.value() / local_day_s, nd.spring.value(),
            sd.spring.value() / local_day_s, sd.spring.value()
        ));
        md.push_str(&format!(
            "| **Verão** | {:.2} dias locais ({:.2} s) | {:.2} dias locais ({:.2} s) |\n",
            nd.summer.value() / local_day_s, nd.summer.value(),
            sd.summer.value() / local_day_s, sd.summer.value()
        ));
        md.push_str(&format!(
            "| **Outono** | {:.2} dias locais ({:.2} s) | {:.2} dias locais ({:.2} s) |\n",
            nd.autumn.value() / local_day_s, nd.autumn.value(),
            sd.autumn.value() / local_day_s, sd.autumn.value()
        ));
        md.push_str(&format!(
            "| **Inverno** | {:.2} dias locais ({:.2} s) | {:.2} dias locais ({:.2} s) |\n\n",
            nd.winter.value() / local_day_s, nd.winter.value(),
            sd.winter.value() / local_day_s, sd.winter.value()
        ));
    }

    md.push_str("## 5. Análise de Intercalação e Frações Contínuas\n\n");
    if let Some(diy) = resolved.intercalation_day_in_year() {
        append_intercalation_section(&mut md, "5.1 Intercalação Dia-no-Ano (Solar / Civil)", diy, "Dias", "Ano");
    }
    if let Some(miy) = resolved.intercalation_month_in_year() {
        append_intercalation_section(&mut md, "5.2 Intercalação Mês-no-Ano (Lunissolar)", miy, "Meses Sinódicos", "Ano");
    }
    if let Some(dim) = resolved.intercalation_day_in_month() {
        append_intercalation_section(&mut md, "5.3 Intercalação Dia-no-Mês (Lunar)", dim, "Dias", "Mês Sinódico");
    }

    md.push_str("## 6. Sistema Lunar e Ciclos de Eclipses (Saros)\n\n");
    let moon_sys = &skel.moon_system;
    if moon_sys.moons().is_empty() {
        md.push_str("Nenhuma lua ou satélite natural vinculado a este corpo planetário.\n\n");
    } else {
        md.push_str("### 6.1 Meses Lunares Derivados por Satélite\n\n");
        md.push_str("| Nome da Lua | ID | Órbita | Mês Sideral | Mês Sinódico | Mês Anomalístico | Mês Dracônico |\n| :--- | :--- | :--- | :--- | :--- | :--- | :--- |\n");
        for moon in moon_sys.moons() {
            let orbit_dir = if moon.is_retrograde_orbit() { "Retrógrada" } else { "Prógrada" };
            let sid = moon.sidereal_month().map(|d| format!("{:.4} dias locais ({:.2} s)", d.value() / local_day_s, d.value())).unwrap_or_else(|| "N/A".to_string());
            let syn = moon.synodic_month().map(|d| format!("{:.4} dias locais ({:.2} s)", d.value() / local_day_s, d.value())).unwrap_or_else(|| "N/A".to_string());
            let anom = moon.anomalistic_month().map(|d| format!("{:.4} dias locais ({:.2} s)", d.value() / local_day_s, d.value())).unwrap_or_else(|| "N/A".to_string());
            let drac = moon.draconic_month().map(|d| format!("{:.4} dias locais ({:.2} s)", d.value() / local_day_s, d.value())).unwrap_or_else(|| "N/A".to_string());

            md.push_str(&format!(
                "| **{}** | `{}` | {} | {} | {} | {} | {} |\n",
                moon.moon_name(), moon.moon_id(), orbit_dir, sid, syn, anom, drac
            ));
        }
        md.push_str("\n");

        md.push_str("### 6.2 Ciclos de Eclipse Tipo-Saros\n\n");
        md.push_str("| Lua | Meses Sinódicos | Meses Dracônicos | Meses Anomalísticos | Duração do Ciclo | Discrepância Dracônica | Discrepância Anomalística |\n| :--- | :--- | :--- | :--- | :--- | :--- | :--- |\n");
        for moon in moon_sys.moons() {
            if let Some(saros) = moon.saros_cycle() {
                md.push_str(&format!(
                    "| **{}** | {} | {} | {} | {:.2} dias locais ({:.2} anos locais) | {:.2} s ({:.4} dias locais) | {:.2} s ({:.4} dias locais) |\n",
                    moon.moon_name(),
                    saros.synodic_month_count,
                    saros.draconic_month_count,
                    saros.anomalistic_month_count,
                    saros.duration.value() / local_day_s,
                    saros.duration.value() / local_year_s,
                    saros.synodic_draconic_discrepancy.value(),
                    saros.synodic_draconic_discrepancy.value() / local_day_s,
                    saros.synodic_anomalistic_discrepancy.value(),
                    saros.synodic_anomalistic_discrepancy.value() / local_day_s
                ));
            } else {
                md.push_str(&format!("| **{}** | *Nenhum ciclo fechado dentro das tolerâncias* | - | - | - | - | - |\n", moon.moon_name()));
            }
        }
        md.push_str("\n");

        if !moon_sys.pair_resonances().is_empty() {
            md.push_str("### 6.3 Ressonâncias de Movimento Médio (Pares de Luas)\n\n");
            md.push_str("| Lua Interna ID | Lua Externa ID | Razão $p:q$ | Ordem | Desvio Relativo |\n| :--- | :--- | :--- |\n");
            for pr in moon_sys.pair_resonances() {
                md.push_str(&format!(
                    "| `{}` | `{}` | {}:{} | {} | {:.6}% |\n",
                    pr.inner_moon_id, pr.outer_moon_id, pr.p, pr.q, pr.resonance_order, pr.deviation * 100.0
                ));
            }
            md.push_str("\n");
        }

        if !moon_sys.laplace_resonances().is_empty() {
            md.push_str("### 6.4 Trios de Ressonância de Laplace\n\n");
            md.push_str("| Lua Interna | Lua Média | Lua Externa | Proporção Harmonizada | Desvio Médio |\n| :--- | :--- | :--- | :--- | :--- |\n");
            for lap in moon_sys.laplace_resonances() {
                md.push_str(&format!(
                    "| `{}` | `{}` | `{}` | {}:{}:{} | {:.6}% |\n",
                    lap.inner_moon_id, lap.middle_moon_id, lap.outer_moon_id,
                    lap.ratio_inner, lap.ratio_middle, lap.ratio_outer, lap.deviation * 100.0
                ));
            }
            md.push_str("\n");
        }
    }

    md.push_str("## 7. Iluminação Polissolar e Sistemas Múltiplos\n\n");
    let poly = skel.polysolar_day();
    md.push_str("| Propriedade Polissolar | Valor |\n| :--- | :--- |\n");
    md.push_str(&format!("| **Classificação do Sistema** | `{:?}` |\n", poly.classification()));
    md.push_str(&format!("| **ID da Estrela Primária** | `{}` |\n", poly.primary_star_id().map(|id| id.to_string()).unwrap_or_else(|| "N/A".to_string())));
    md.push_str(&format!(
        "| **Grande Ciclo Polissolar** | {} |\n\n",
        poly.grand_cycle()
            .map(|d| format!("{:.4} s ({:.4} dias locais)", d.value(), d.value() / local_day_s))
            .unwrap_or_else(|| "N/A (Estrela única ou não ressonante)".to_string())
    ));

    md.push_str("### 7.1 Componentes Estelares e Relevância Luminosa\n\n");
    md.push_str("| Estrela | ID | Distância (UA) | Irradiância ($W/m^2$) | Magnitude Aparente ($m$) | Primária? | Sol Funcional ($m \\le -15$)? | Dia Solar Individual |\n| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |\n");
    for comp in poly.components() {
        let dist_au = comp.distance.value() / ASTRONOMICAL_UNIT;
        let day_str = comp
            .individual_solar_day
            .map(|d| format!("{:.4} s ({:.4} dias locais)", d.value(), d.value() / local_day_s))
            .unwrap_or_else(|| "Síncrono/Inexistente".to_string());
        md.push_str(&format!(
            "| **{}** | `{}` | {:.4} | {:.4e} | {:.2} | `{}` | `{}` | {} |\n",
            comp.star_name, comp.star_id, dist_au, comp.irradiance.value(), comp.apparent_magnitude, comp.is_primary, comp.is_relevant, day_str
        ));
    }
    md.push_str("\n");

    md.push_str("## 8. Simulação de Resolução Temporal (Ticks de Exemplo)\n\n");
    md.push_str("### 8.1 Estado no Epoch ($t = 0$ s)\n\n");
    append_tick_table(&mut md, tick_epoch);

    md.push_str("\n### 8.2 Estado após 1 Ano Orbital\n\n");
    append_tick_table(&mut md, tick_one_year);

    md.push_str("\n---\n*Relatório gerado automaticamente por Chronicon Engine.*\n");
    md
}

fn append_intercalation_section(
    md: &mut String,
    title: &str,
    analysis: &IntercalationAnalysis,
    unit_name: &str,
    container_name: &str,
) {
    md.push_str(&format!("### {}\n\n", title));
    md.push_str(&format!("* **{} Exatos por {}:** `{:.10}`\n", unit_name, container_name, analysis.exact_units_per_container()));
    md.push_str(&format!("* **Duração da Unidade:** `{:.4}` s\n", analysis.unit_duration().value()));
    md.push_str(&format!("* **Duração do Recipiente:** `{:.4}` s\n\n", analysis.container_duration().value()));

    let p = analysis.primary_rule();
    md.push_str("#### Regra Primária\n\n");
    md.push_str(&format!(
        "| Base por Ciclo | Unidades Bissextas | Ciclo ({}) | Média ({}/{}) | Desvio por Milênio | {} para 1 {} de Desvio |\n| :--- | :--- | :--- | :--- | :--- | :--- |\n",
        container_name, unit_name, container_name, container_name, unit_name
    ));
    md.push_str(&format!(
        "| {} | {} | {} | {:.6} | {:.4} {} | {} {} |\n\n",
        p.base_units_per_container(),
        p.leap_units(),
        p.cycle_containers(),
        p.mean_units_per_container(),
        p.drift_units_per_thousand_containers(),
        unit_name,
        p.containers_per_unit_drift()
            .map(|v| format!("{:.2}", v))
            .unwrap_or_else(|| "0".to_string()),
        container_name
    ));

    let r = analysis.refined_rule();
    md.push_str("#### Regra Refinada (Com Correção Secundária)\n\n");
    md.push_str(&format!(
        "* **Ajuste Secundário:** {}\n",
        r.secondary_correction()
            .map(|s| format!("`{:?}` de {} {} a cada {} {}", s.direction(), s.leap_units(), unit_name, s.cycle_containers(), container_name))
            .unwrap_or_else(|| "Nenhum necessário".to_string())
    ));
    md.push_str(&format!("* **Regra Combinada Final:** {} {} bissextos a cada {} {}\n", r.total_leap_units(), unit_name, r.total_cycle_containers(), container_name));
    md.push_str(&format!("* **Média Final:** `{:.8}` {}/{}\n", r.mean_units_per_container(), unit_name, container_name));
    md.push_str(&format!("* **Deriva Residual por Milênio:** `{:.6}` {}\n", r.drift_units_per_thousand_containers(), unit_name));
    md.push_str(&format!(
        "* **{} para 1 {} de Deriva:** {}\n\n",
        container_name,
        unit_name,
        r.containers_per_unit_drift()
            .map(|v| format!("{:.2}", v))
            .unwrap_or_else(|| "Perfeita (Sem deriva detectável)".to_string())
    ));

    md.push_str("#### Convergentes de Fração Contínua Candidatos\n\n");
    md.push_str(&format!("| Numerador (Bissextos) | Denominador ({}) | Média Calculada | Erro Residual | Desvio / 1000 {} |\n| :--- | :--- | :--- | :--- | :--- |\n", container_name, container_name));
    for cand in analysis.candidate_cycles() {
        let err = cand.mean_units_per_container() - analysis.exact_units_per_container();
        md.push_str(&format!(
            "| {} | {} | {:.8} | {:.6e} | {:.4} {} |\n",
            cand.leap_units(),
            cand.cycle_containers(),
            cand.mean_units_per_container(),
            err,
            cand.drift_units_per_thousand_containers(),
            unit_name
        ));
    }
    md.push_str("\n");
}

fn append_tick_table(md: &mut String, tick: &chronicon_app::tick::CalendarTick) {
    md.push_str("| Campo Temporal | Valor Resolvido |\n| :--- | :--- |\n");
    md.push_str(&format!("| **Instante Físico** | `{:.4}` s desde época |\n", tick.instant_seconds));
    md.push_str(&format!("| **Índice do Ano** | `{}` |\n", tick.year_index));
    md.push_str(&format!("| **Dia no Ano (Fracionário)** | `{:.6}` |\n", tick.day_in_year));
    md.push_str(&format!("| **Índice do Dia no Ano** | `{}` |\n", tick.day_index));
    md.push_str(&format!("| **Mês no Ano** | {} |\n", tick.month_in_year.map(|m| m.to_string()).unwrap_or_else(|| "N/A".to_string())));
    md.push_str(&format!("| **Ano Bissexto?** | `{}` |\n", tick.is_leap_year));
    md.push_str(&format!("| **Fronteira de Intercalação de Dia?** | `{}` |\n", tick.is_intercalation_boundary));
    md.push_str(&format!("| **Fronteira de Mês Bissexto?** | {} |\n", tick.is_leap_month_boundary.map(|b| b.to_string()).unwrap_or_else(|| "N/A".to_string())));
    md.push_str(&format!("| **Estação Hemisfério Norte** | `{:?}` |\n", tick.season.northern_season));
    md.push_str(&format!("| **Estação Hemisfério Sul** | `{:?}` |\n", tick.season.southern_season));
    md.push_str(&format!("| **Progresso na Estação** | `{:.2}%` |\n", tick.season.season_progress * 100.0));
    md.push_str(&format!("| **Progresso Orbital** | `{:.2}%` |\n", tick.season.orbital_progress * 100.0));

    if !tick.moon_phases.is_empty() {
        md.push_str("\n**Fases Lunares no Instante:**\n\n");
        md.push_str("| Lua | Ângulo de Fase (rad) | Iluminação (%) | Fase Lunar |\n| :--- | :--- | :--- | :--- |\n");
        for moon in &tick.moon_phases {
            md.push_str(&format!(
                "| **{}** | {:.4} | {:.2}% | `{:?}` |\n",
                moon.moon_name,
                moon.phase_angle.value(),
                moon.illumination_fraction * 100.0,
                moon.phase_name
            ));
        }
    }
}