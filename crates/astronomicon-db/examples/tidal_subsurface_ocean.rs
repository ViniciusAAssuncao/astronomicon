use astronomicon_app::climate::resolve_global_mean_temperature;
use astronomicon_app::geophysics::resolve_planetary_core;
use astronomicon_app::hydrosphere::resolve_hydrosphere_diagnostics;
use astronomicon_app::tidal::resolve_tidal_diagnostics;
use astronomicon_core::units::constants::{ ASTRONOMICAL_UNIT, SOLAR_MASS, SOLAR_RADIUS };
use astronomicon_core::units::Duration;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "===================================================================================================="
    );
    println!(
        "             ASTRONOMICON - RELATÓRIO DE AQUECIMENTO POR MARÉ E OCEANOS SUBSUPERFICIAIS             "
    );
    println!(
        "                                   (Caso Canônico: Europa / Júpiter)                                "
    );
    println!(
        "===================================================================================================="
    );

    let pool = astronomicon_db::save::initialize_save("sqlite::memory:?cache=shared").await?;

    let system_id = Uuid::new_v4();
    let sun_id = Uuid::new_v4();
    let jupiter_id = Uuid::new_v4();
    let europa_id = Uuid::new_v4();
    let europa_hydro_id = Uuid::new_v4();

    sqlx
        ::query(
            "INSERT INTO star_systems (id, name, right_ascension_rad, declination_rad, distance_from_sol_m) \
         VALUES (?, ?, ?, ?, ?)"
        )
        .bind(system_id.to_string())
        .bind("Sistema Solar")
        .bind(0.0)
        .bind(0.0)
        .bind(0.0)
        .execute(&pool).await?;

    sqlx
        ::query(
            "INSERT INTO stars (id, star_system_id, name, kind, mass_kg, radius_m, effective_temperature_k, \
         rotation_period_s, axial_tilt_rad, oblateness_j2) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(sun_id.to_string())
        .bind(system_id.to_string())
        .bind("Sol")
        .bind("Star")
        .bind(SOLAR_MASS)
        .bind(SOLAR_RADIUS)
        .bind(5778.0)
        .bind(2160000.0)
        .bind(0.1265)
        .bind(0.00002)
        .execute(&pool).await?;

    sqlx
        ::query(
            "INSERT INTO planets (id, star_system_id, parent_star_id, name, kind, mass_kg, equatorial_radius_m, \
         polar_radius_m, rotation_period_s, axial_tilt_rad, geometric_albedo, bond_albedo, thermal_inertia, \
         solstice_true_anomaly_rad, semi_major_axis_m, eccentricity, inclination_rad, longitude_ascending_node_rad, \
         argument_periapsis_rad, mean_anomaly_at_epoch_rad, oblateness_j2, core_mass_fraction, \
         radioactive_heating_rate, love_number_k2, tidal_dissipation_factor_q) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(jupiter_id.to_string())
        .bind(system_id.to_string())
        .bind(sun_id.to_string())
        .bind("Júpiter")
        .bind("GasGiant")
        .bind(1.89813e27)
        .bind(71492000.0)
        .bind(66854000.0)
        .bind(35730.0)
        .bind(0.0546)
        .bind(0.52)
        .bind(0.503)
        .bind(0.9)
        .bind(0.0)
        .bind(5.2044 * ASTRONOMICAL_UNIT)
        .bind(0.0484)
        .bind(0.0227)
        .bind(1.75)
        .bind(4.77)
        .bind(0.34)
        .bind(0.01475)
        .bind(0.05)
        .bind(0.0)
        .bind(0.565)
        .bind(35000.0)
        .execute(&pool).await?;

    sqlx
        ::query(
            "INSERT INTO planets (id, star_system_id, parent_planet_id, name, kind, mass_kg, equatorial_radius_m, \
         polar_radius_m, rotation_period_s, axial_tilt_rad, geometric_albedo, bond_albedo, thermal_inertia, \
         solstice_true_anomaly_rad, semi_major_axis_m, eccentricity, inclination_rad, longitude_ascending_node_rad, \
         argument_periapsis_rad, mean_anomaly_at_epoch_rad, oblateness_j2, core_mass_fraction, \
         radioactive_heating_rate, love_number_k2, tidal_dissipation_factor_q) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(europa_id.to_string())
        .bind(system_id.to_string())
        .bind(jupiter_id.to_string())
        .bind("Europa")
        .bind("IcyBody")
        .bind(4.7998e22)
        .bind(1560800.0)
        .bind(1560800.0)
        .bind(306822.0)
        .bind(0.0017)
        .bind(0.67)
        .bind(0.62)
        .bind(0.2)
        .bind(0.0)
        .bind(670900000.0)
        .bind(0.009)
        .bind(0.0082)
        .bind(0.0)
        .bind(0.0)
        .bind(0.0)
        .bind(0.0001)
        .bind(0.12)
        .bind(0.15)
        .bind(0.25)
        .bind(35.0)
        .execute(&pool).await?;

    sqlx
        ::query(
            "INSERT INTO hydrospheres (id, planet_id, average_depth_m, surface_coverage_fraction, salinity_or_solute_mass_fraction) \
         VALUES (?, ?, ?, ?, ?)"
        )
        .bind(europa_hydro_id.to_string())
        .bind(europa_id.to_string())
        .bind(100000.0)
        .bind(1.0)
        .bind(0.035)
        .execute(&pool).await?;

    sqlx
        ::query(
            "INSERT INTO hydrosphere_components (hydrosphere_id, formula, percentage) VALUES (?, ?, ?)"
        )
        .bind(europa_hydro_id.to_string())
        .bind("H2O")
        .bind(100.0)
        .execute(&pool).await?;

    let epoch_zero = Duration::new(0.0);
    let sim_age = Duration::new(4.5e9 * 365.25 * 86400.0);

    println!("[SEÇÃO 1] CONFIGURAÇÃO ORBITAL E DADOS GEOFÍSICOS DE EUROPA");
    println!(
        "----------------------------------------------------------------------------------------------------"
    );
    println!("  Corpo Primário (Pai)        : Júpiter (1.8981e27 kg)");
    println!("  Massa de Europa             : 4.7998e22 kg (0.008 M_Terra)");
    println!("  Raio Equatorial             : 1560.80 km");
    println!("  Semi-eixo Maior Orbital (a) : 670,900.00 km");
    println!("  Excentricidade Orbital (e)  : 0.0090 (Forçada pela Ressonância de Laplace)");
    println!("  Período Orbital / Rotação   : 3.5512 dias (Travado Sincronamente)");
    println!("  Camada de Água Total (H2O)  : 100.00 km (100% de Cobertura Global)");
    println!("  Salinidade da Água          : 3.50 %");
    println!();

    println!("[SEÇÃO 2] DIAGNÓSTICO DE FORÇAS DE MARÉ E DISSIPAÇÃO TÉRMICA");
    println!(
        "----------------------------------------------------------------------------------------------------"
    );
    let tidal_diag = resolve_tidal_diagnostics(&pool, europa_id, sim_age, epoch_zero).await?;

    println!("  Número de Love k₂ Utilizado : {:>12.4}", tidal_diag.love_number_k2);
    println!("  Fator de Dissipação Q       : {:>12.2}", tidal_diag.dissipation_factor_q);
    println!(
        "  Escala de Tempo de Travamento: {:>12.4e} anos",
        tidal_diag.tidal_locking_timescale.value() / (365.25 * 86400.0)
    );
    println!("  Status de Acoplamento Maré  : {}", if tidal_diag.is_tidally_locked {
        "ACOPLADO (SÍNCRONO)"
    } else {
        "NÃO ACOPLADO"
    });
    println!(
        "  Potência Térmica Dissipada  : {:>12.4e} W ({:.4} GW)",
        tidal_diag.tidal_heating_energy.value(),
        tidal_diag.tidal_heating_energy.value() * 1e-9
    );
    println!(
        "  Fluxo Térmico de Maré (Surf): {:>12.4e} W/m² ({:.2} mW/m²)",
        tidal_diag.tidal_surface_heat_flux.value(),
        tidal_diag.tidal_surface_heat_flux.value() * 1e3
    );
    println!();

    println!("[SEÇÃO 3] BALANÇO DE FLUXO GEOTÉRMICO TOTAL NA BASE DA CROSTA");
    println!(
        "----------------------------------------------------------------------------------------------------"
    );
    let core_diag = resolve_planetary_core(&pool, europa_id, sim_age, epoch_zero).await?;

    println!(
        "  Fluxo Radiogênico do Manto  : {:>12.4e} W/m² ({:.2} mW/m²)",
        core_diag.radiogenic_heat_flux.value(),
        core_diag.radiogenic_heat_flux.value() * 1e3
    );
    println!(
        "  Fluxo do Limite Núcleo-Manto: {:>12.4e} W/m² ({:.2} mW/m²)",
        core_diag.cmb_heat_flux.value(),
        core_diag.cmb_heat_flux.value() * 1e3
    );
    println!(
        "  Fluxo de Dissipação de Maré : {:>12.4e} W/m² ({:.2} mW/m²)",
        core_diag.tidal_heat_flux.value(),
        core_diag.tidal_heat_flux.value() * 1e3
    );
    println!(
        "  --------------------------------------------------------------------------------------------------"
    );
    println!(
        "  FLUXO GEOTÉRMICO TOTAL (q_geo): {:>10.4e} W/m² ({:.2} mW/m²)",
        core_diag.total_surface_heat_flux.value(),
        core_diag.total_surface_heat_flux.value() * 1e3
    );
    let tidal_pct =
        (core_diag.tidal_heat_flux.value() / core_diag.total_surface_heat_flux.value().max(1e-12)) *
        100.0;
    println!("  Contribuição da Maré no Fluxo : {:>10.2} %", tidal_pct);
    println!();

    println!("[SEÇÃO 4] ESTRUTURA TÉRMICA DA HIDROSFERA E OCEANO SUBSUPERFICIAL");
    println!(
        "----------------------------------------------------------------------------------------------------"
    );
    let t_surf = resolve_global_mean_temperature(&pool, europa_id, sim_age, epoch_zero).await?;
    let hydro_diag = resolve_hydrosphere_diagnostics(
        &pool,
        europa_id,
        sim_age,
        epoch_zero
    ).await?.unwrap();

    println!(
        "  Temperatura de Superfície   : {:>8.2} K ({:>6.2} °C)",
        t_surf.value(),
        t_surf.value() - 273.15
    );
    println!(
        "  Ponto de Congelamento       : {:>8.2} K ({:>6.2} °C)",
        hydro_diag.surface_freezing_point.value(),
        hydro_diag.surface_freezing_point.value() - 273.15
    );
    println!(
        "  Ponto de Ebulição           : {:>8.2} K ({:>6.2} °C)",
        hydro_diag.surface_boiling_point.value(),
        hydro_diag.surface_boiling_point.value() - 273.15
    );
    println!("  Estado Dominante Superficial: {:?}", hydro_diag.dominant_state);
    println!(
        "  --------------------------------------------------------------------------------------------------"
    );
    println!(
        "  Espessura da Crosta de Gelo : {:>10.2} km",
        hydro_diag.ice_thickness.value() * 1e-3
    );
    println!("  Profundidade do Oceano Líq. : {:>10.2} km", hydro_diag.liquid_depth.value() * 1e-3);
    println!("  Massa Total da Hidrosfera   : {:>10.4e} kg", hydro_diag.total_mass.value());
    println!(
        "  --------------------------------------------------------------------------------------------------"
    );
    println!("  Status do Oceano Subsuperf. : {}", if hydro_diag.is_subsurface_ocean {
        "SIM (ATIVO)"
    } else {
        "NÃO"
    });
    println!("  Completamente Congelado     : {}", if hydro_diag.is_completely_frozen {
        "SIM"
    } else {
        "NÃO"
    });
    println!("  Completamente Líquido       : {}", if hydro_diag.is_completely_liquid {
        "SIM"
    } else {
        "NÃO"
    });
    println!();

    assert!(hydro_diag.is_subsurface_ocean);
    assert!(hydro_diag.ice_thickness.value() > 0.0);
    assert!(hydro_diag.liquid_depth.value() > 0.0);
    assert!(!hydro_diag.is_completely_frozen);

    println!(
        "===================================================================================================="
    );
    println!(
        "Status da Validação: ACOPLAMENTO TERMO-MECÂNICO DE MARÉ E GÊNESE DO OCEANO SUBSUPERFICIAL CONFIRMADOS."
    );
    println!(
        "===================================================================================================="
    );

    Ok(())
}
