use astronomicon_app::{
    resolve_hill_sphere, resolve_minor_planet_diagnostics, resolve_net_gravitational_acceleration,
    resolve_roche_limits,
};
use astronomicon_core::units::constants::{ASTRONOMICAL_UNIT, SOLAR_MASS, SOLAR_RADIUS};
use astronomicon_core::units::{Duration, Position};
use astronomicon_db::connection::open_pool;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = open_pool("sqlite::memory:?cache=shared").await?;

    sqlx::query("INSERT INTO universe_state (id, seconds_since_j2000_epoch) VALUES (1, 0.0)")
        .execute(&pool)
        .await?;

    let system_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO star_systems (id, name, right_ascension_rad, declination_rad, distance_from_sol_m) \
         VALUES (?, 'Solarium Prime', 0.0, 0.0, 1.0e16)",
    )
    .bind(system_id.to_string())
    .execute(&pool)
    .await?;

    let star_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO stars (id, star_system_id, name, kind, mass_kg, radius_m, effective_temperature_k, \
         rotation_period_s, axial_tilt_rad, oblateness_j2, metallicity) \
         VALUES (?, ?, 'Solarium A', 'Star', ?, ?, 5778.0, 2160000.0, 0.1, 0.00005, 0.0)",
    )
    .bind(star_id.to_string())
    .bind(system_id.to_string())
    .bind(SOLAR_MASS)
    .bind(SOLAR_RADIUS)
    .execute(&pool)
    .await?;

    let apophis_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO minor_planets (id, star_system_id, parent_star_id, name, spectral_type, mass_kg, \
         axis_a_m, axis_b_m, axis_c_m, rotation_period_s, axial_tilt_rad, macroporosity, geometric_albedo, \
         bond_albedo, semi_major_axis_m, eccentricity, inclination_rad, longitude_ascending_node_rad, \
         argument_periapsis_rad, mean_anomaly_at_epoch_rad) \
         VALUES (?, ?, ?, 'Ryugu-Alpha', 'C', 4.5e11, 500.0, 450.0, 400.0, 7200.0, 0.2, 0.45, 0.045, 0.02, \
         ?, 0.15, 0.05, 0.0, 0.0, 0.0)",
    )
    .bind(apophis_id.to_string())
    .bind(system_id.to_string())
    .bind(star_id.to_string())
    .bind(1.2 * ASTRONOMICAL_UNIT)
    .execute(&pool)
    .await?;

    let psyche_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO minor_planets (id, star_system_id, parent_star_id, name, spectral_type, mass_kg, \
         axis_a_m, axis_b_m, axis_c_m, rotation_period_s, axial_tilt_rad, macroporosity, geometric_albedo, \
         bond_albedo, semi_major_axis_m, eccentricity, inclination_rad, longitude_ascending_node_rad, \
         argument_periapsis_rad, mean_anomaly_at_epoch_rad) \
         VALUES (?, ?, ?, 'Ferrum-16', 'M', 2.4e19, 140000.0, 115000.0, 95000.0, 15000.0, 0.15, 0.08, 0.15, 0.10, \
         ?, 0.12, 0.08, 0.2, 0.4, 0.0)",
    )
    .bind(psyche_id.to_string())
    .bind(system_id.to_string())
    .bind(star_id.to_string())
    .bind(2.8 * ASTRONOMICAL_UNIT)
    .execute(&pool)
    .await?;

    let eros_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO minor_planets (id, star_system_id, parent_star_id, name, spectral_type, mass_kg, \
         axis_a_m, axis_b_m, axis_c_m, rotation_period_s, axial_tilt_rad, macroporosity, geometric_albedo, \
         bond_albedo, semi_major_axis_m, eccentricity, inclination_rad, longitude_ascending_node_rad, \
         argument_periapsis_rad, mean_anomaly_at_epoch_rad) \
         VALUES (?, ?, ?, 'Eros-Prime', 'S', 6.68e15, 17000.0, 5500.0, 5500.0, 18900.0, 0.4, 0.20, 0.25, 0.15, \
         ?, 0.22, 0.18, 0.5, 1.2, 0.0)",
    )
    .bind(eros_id.to_string())
    .bind(system_id.to_string())
    .bind(star_id.to_string())
    .bind(1.45 * ASTRONOMICAL_UNIT)
    .execute(&pool)
    .await?;

    let active_comet_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO minor_planets (id, star_system_id, parent_star_id, name, spectral_type, mass_kg, \
         axis_a_m, axis_b_m, axis_c_m, rotation_period_s, axial_tilt_rad, macroporosity, geometric_albedo, \
         bond_albedo, semi_major_axis_m, eccentricity, inclination_rad, longitude_ascending_node_rad, \
         argument_periapsis_rad, mean_anomaly_at_epoch_rad) \
         VALUES (?, ?, ?, 'Halley-Beta (Ativo)', 'D', 2.2e14, 8000.0, 4000.0, 4000.0, 180000.0, 0.3, 0.60, 0.04, 0.03, \
         ?, 0.95, 0.3, 0.0, 0.0, 0.0)",
    )
    .bind(active_comet_id.to_string())
    .bind(system_id.to_string())
    .bind(star_id.to_string())
    .bind(15.0 * ASTRONOMICAL_UNIT)
    .execute(&pool)
    .await?;

    let inactive_comet_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO minor_planets (id, star_system_id, parent_star_id, name, spectral_type, mass_kg, \
         axis_a_m, axis_b_m, axis_c_m, rotation_period_s, axial_tilt_rad, macroporosity, geometric_albedo, \
         bond_albedo, semi_major_axis_m, eccentricity, inclination_rad, longitude_ascending_node_rad, \
         argument_periapsis_rad, mean_anomaly_at_epoch_rad) \
         VALUES (?, ?, ?, 'Oort-Wanderer (Inativo)', 'P', 1.0e13, 2000.0, 1500.0, 1200.0, 86400.0, 0.0, 0.50, 0.03, 0.02, \
         ?, 0.01, 0.1, 0.0, 0.0, 0.0)",
    )
    .bind(inactive_comet_id.to_string())
    .bind(system_id.to_string())
    .bind(star_id.to_string())
    .bind(45.0 * ASTRONOMICAL_UNIT)
    .execute(&pool)
    .await?;

    let universe_epoch = Duration::new(0.0);
    let at_epoch = Duration::new(0.0);

    let minor_planet_ids = [
        ("Ryugu-Alpha (Asteroide C - Rubble Pile)", apophis_id),
        ("Ferrum-16 (Asteroide M - Metálico Massivo)", psyche_id),
        ("Eros-Prime (Asteroide S - Silicato Alongado)", eros_id),
        ("Halley-Beta (Cometa D - Periélio 0.75 AU)", active_comet_id),
        ("Oort-Wanderer (Cometa P - Afélio 45.0 AU)", inactive_comet_id),
    ];

    println!("{}", "=".repeat(110));
    println!(
        "{:^110}",
        "ASTRONOMICON - RELATÓRIO FÍSICO DE CORPOS MENORES (ASTEROIDES E COMETAS)"
    );
    println!("{}", "=".repeat(110));

    for (label, mp_id) in minor_planet_ids {
        let diag =
            resolve_minor_planet_diagnostics(&pool, mp_id, universe_epoch, at_epoch).await?;
        let hill = resolve_hill_sphere(&pool, &system_id, &mp_id).await?;
        let roche = resolve_roche_limits(&pool, &system_id, &star_id, &mp_id).await?;

        println!("{}", "-".repeat(110));
        println!("CORPO MENOR: {:<45} | ID: {}", label, mp_id);
        println!("{}", "-".repeat(110));

        println!("[1] MORFOLOGIA E GEOMETRIA TRIAXIAL:");
        println!(
            "  Raio Esférico Equivalente      : {:>12.2} m ({:.3} km)",
            diag.equivalent_spherical_radius.value(),
            diag.equivalent_spherical_radius.value() / 1000.0
        );
        println!(
            "  Volume do Elipsoide Triaxial   : {:>12.4e} m³",
            diag.volume_m3
        );
        println!(
            "  Área Superficial (Knud-Thomsen): {:>12.4e} m²",
            diag.surface_area_m2
        );
        println!();

        println!("[2] REOLOGIA DE RUBBLE PILE E ESTABILIDADE ROTACIONAL:");
        println!(
            "  Densidade de Grão (Espectral)  : {:>12.2} kg/m³",
            diag.rubble_pile.grain_density.value()
        );
        println!(
            "  Macroporosidade                : {:>12.2} %",
            diag.rubble_pile.macroporosity * 100.0
        );
        println!(
            "  Densidade Bulk (Maciça)        : {:>12.2} kg/m³",
            diag.rubble_pile.bulk_density.value()
        );
        println!(
            "  Período de Rotação Crítico     : {:>12.2} s ({:.2} h)",
            diag.rubble_pile.critical_rotation_period.value(),
            diag.rubble_pile.critical_rotation_period.value() / 3600.0
        );
        println!(
            "  Perda de Massa por Centrifugação: {}",
            if diag.rubble_pile.is_centrifugal_shedding {
                "SIM (Instável - Superou Barreira de Spin)"
            } else {
                "NÃO (Estável / Coeso)"
            }
        );
        println!();

        println!("[3] DINÂMICA GRAVITACIONAL E LIMITES ORBITAIS:");
        println!(
            "  Raio da Esfera de Hill         : {:>12.4e} m ({:.2} km)",
            hill.value(),
            hill.value() / 1000.0
        );
        println!(
            "  Limite de Roche Rígido c/ Sol  : {:>12.4e} m ({:.2} km)",
            roche.rigid.value(),
            roche.rigid.value() / 1000.0
        );
        println!(
            "  Limite de Roche Fluido c/ Sol  : {:>12.4e} m ({:.2} km)",
            roche.fluid.value(),
            roche.fluid.value() / 1000.0
        );
        println!();

        println!("[4] ATIVIDADE COMETÁRIA E SUBLIMAÇÃO DE VOLÁTEIS:");
        if let Some(comet) = diag.cometary_activity {
            println!(
                "  Status de Atividade Cometária  : {}",
                if comet.is_active {
                    "ATIVO (Sublimação Termodinâmica Sustentada)"
                } else {
                    "INATIVO / DORMIDAL (Frio Excessivo)"
                }
            );
            println!(
                "  Taxa de Emissão de H2O         : {:>12.4e} kg/s",
                comet.water_outgassing_rate.value()
            );
            println!(
                "  Taxa de Emissão de CO2         : {:>12.4e} kg/s",
                comet.co2_outgassing_rate.value()
            );
            println!(
                "  Produção Total de Gás          : {:>12.4e} kg/s",
                comet.total_gas_production_rate.value()
            );
            println!(
                "  Raio da Coma Efêmera           : {:>12.2} m ({:.2} km)",
                comet.coma_radius.value(),
                comet.coma_radius.value() / 1000.0
            );
            println!(
                "  Comprimento da Cauda de Íons   : {:>12.4e} m ({:.2} km)",
                comet.ion_tail_length.value(),
                comet.ion_tail_length.value() / 1000.0
            );
            println!(
                "  Comprimento da Cauda de Poeira : {:>12.4e} m ({:.2} km)",
                comet.dust_tail_length.value(),
                comet.dust_tail_length.value() / 1000.0
            );
            println!(
                "  Força de Arrasto Iônico        : {:>12.4e} N",
                comet.ion_tail_drag_force_n
            );
            println!(
                "  Força de Pressão de Radiação   : {:>12.4e} N",
                comet.dust_radiation_force_n
            );
        } else {
            println!("  Sem atividade cometária detectada.");
        }
        println!();
    }

    let acc_sample = resolve_net_gravitational_acceleration(
        &pool,
        &system_id,
        Position::from_components(ASTRONOMICAL_UNIT, 0.0, 0.0),
        universe_epoch,
    )
    .await?;

    println!("{}", "-".repeat(110));
    println!("[5] ACELERAÇÃO GRAVITACIONAL TOTAL DO SISTEMA EM 1.0 AU:");
    println!(
        "  Vetor Gravitacional Líquido   : ({:.4e}, {:.4e}, {:.4e}) m/s²",
        acc_sample.raw().0,
        acc_sample.raw().1,
        acc_sample.raw().2
    );
    println!(
        "  Magnitude da Aceleração       : {:.4e} m/s²",
        acc_sample.magnitude().value()
    );

    println!("{}", "=".repeat(110));
    println!("Status da Validação: TODOS OS DIAGNÓSTICOS DE CORPOS MENORES CONCLUÍDOS COM SUCESSO.");
    println!("{}", "=".repeat(110));

    Ok(())
}