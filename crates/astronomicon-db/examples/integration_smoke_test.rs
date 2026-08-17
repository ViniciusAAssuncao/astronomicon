use astronomicon_app::ephemeris::resolve_system_positions;
use astronomicon_app::gravity::{
    resolve_barycenter_stability, resolve_hill_sphere, resolve_kozai_lidov_diagnostic,
    resolve_net_gravitational_acceleration,
};
use astronomicon_app::lagrange::resolve_lagrange_points;
use astronomicon_app::resonance::resolve_orbital_resonance;
use astronomicon_core::units::constants::ASTRONOMICAL_UNIT;
use astronomicon_core::units::{Duration, Position};
use std::f64::consts::PI;
use uuid::Uuid;

const SOLAR_MASS_KG: f64 = 1.98847e30;
const SOLAR_RADIUS_M: f64 = 6.957e8;
const EARTH_MASS_KG: f64 = 5.9722e24;
const EARTH_RADIUS_M: f64 = 6.371e6;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "===================================================================================================="
    );
    println!(
        "                     ASTRONOMICON - TESTE DE INTEGRAÇÃO DE PIPELINE COMPLETO (SMOKE TEST)           "
    );
    println!(
        "===================================================================================================="
    );

    let pool = astronomicon_db::save::initialize_save("sqlite::memory:?cache=shared").await?;

    let system_id = Uuid::new_v4();
    let helca_id = Uuid::new_v4();
    let asdi_id = Uuid::new_v4();
    let barycenter_id = Uuid::new_v4();
    let nelica_id = Uuid::new_v4();
    let planet1_id = Uuid::new_v4();
    let planet2_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO star_systems (id, name, right_ascension_rad, declination_rad, distance_from_sol_m) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(system_id.to_string())
    .bind("Sistema Zód Integrado")
    .bind(1.25)
    .bind(-0.45)
    .bind(4.12e17)
    .execute(&pool)
    .await?;

    sqlx::query(
        "INSERT INTO stars (id, star_system_id, name, kind, mass_kg, radius_m, effective_temperature_k, \
         rotation_period_s, axial_tilt_rad, oblateness_j2) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(helca_id.to_string())
    .bind(system_id.to_string())
    .bind("Hélca")
    .bind("Star")
    .bind(1.05 * SOLAR_MASS_KG)
    .bind(1.02 * SOLAR_RADIUS_M)
    .bind(5820.0)
    .bind(2.1e6)
    .bind(0.05)
    .bind(0.005)
    .execute(&pool)
    .await?;

    sqlx::query(
        "INSERT INTO stars (id, star_system_id, name, kind, mass_kg, radius_m, effective_temperature_k, \
         rotation_period_s, axial_tilt_rad) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(asdi_id.to_string())
    .bind(system_id.to_string())
    .bind("Ásdi")
    .bind("Star")
    .bind(0.72 * SOLAR_MASS_KG)
    .bind(0.75 * SOLAR_RADIUS_M)
    .bind(4600.0)
    .bind(3.0e6)
    .bind(0.08)
    .execute(&pool)
    .await?;

    sqlx::query(
        "INSERT INTO barycenters (id, star_system_id, name, primary_star_id, secondary_star_id, \
         internal_semi_major_axis_m, internal_eccentricity, internal_inclination_rad, \
         internal_longitude_ascending_node_rad, internal_argument_periapsis_rad, internal_mean_anomaly_at_epoch_rad) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(barycenter_id.to_string())
    .bind(system_id.to_string())
    .bind("Baricentro Hélca-Ásdi")
    .bind(helca_id.to_string())
    .bind(asdi_id.to_string())
    .bind(1.80 * ASTRONOMICAL_UNIT)
    .bind(0.12)
    .bind(0.0)
    .bind(0.0)
    .bind(0.0)
    .bind(0.0)
    .execute(&pool)
    .await?;

    sqlx::query(
        "INSERT INTO stars (id, star_system_id, parent_barycenter_id, name, kind, mass_kg, radius_m, \
         effective_temperature_k, semi_major_axis_m, eccentricity, inclination_rad, \
         longitude_ascending_node_rad, argument_periapsis_rad, mean_anomaly_at_epoch_rad) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(nelica_id.to_string())
    .bind(system_id.to_string())
    .bind(barycenter_id.to_string())
    .bind("Nélica")
    .bind("WhiteDwarf")
    .bind(0.60 * SOLAR_MASS_KG)
    .bind(0.012 * SOLAR_RADIUS_M)
    .bind(18500.0)
    .bind(75.0 * ASTRONOMICAL_UNIT)
    .bind(0.05)
    .bind(5.0 * PI / 180.0)
    .bind(25.0 * PI / 180.0)
    .bind(70.0 * PI / 180.0)
    .bind(0.0)
    .execute(&pool)
    .await?;

    sqlx::query(
        "INSERT INTO planets (id, star_system_id, parent_star_id, name, kind, mass_kg, equatorial_radius_m, \
         semi_major_axis_m, eccentricity, inclination_rad, longitude_ascending_node_rad, \
         argument_periapsis_rad, mean_anomaly_at_epoch_rad) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(planet1_id.to_string())
    .bind(system_id.to_string())
    .bind(helca_id.to_string())
    .bind("Zód I (Interno)")
    .bind("Telluric")
    .bind(3.0 * EARTH_MASS_KG)
    .bind(1.4 * EARTH_RADIUS_M)
    .bind(0.40 * ASTRONOMICAL_UNIT)
    .bind(0.02)
    .bind(0.01)
    .bind(0.0)
    .bind(0.5)
    .bind(0.0)
    .execute(&pool)
    .await?;

    sqlx::query(
        "INSERT INTO planets (id, star_system_id, parent_star_id, name, kind, mass_kg, equatorial_radius_m, \
         semi_major_axis_m, eccentricity, inclination_rad, longitude_ascending_node_rad, \
         argument_periapsis_rad, mean_anomaly_at_epoch_rad) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(planet2_id.to_string())
    .bind(system_id.to_string())
    .bind(helca_id.to_string())
    .bind("Zód II (Externo)")
    .bind("Telluric")
    .bind(2.0 * EARTH_MASS_KG)
    .bind(1.2 * EARTH_RADIUS_M)
    .bind(0.635 * ASTRONOMICAL_UNIT)
    .bind(0.03)
    .bind(0.015)
    .bind(0.1)
    .bind(1.2)
    .bind(0.0)
    .execute(&pool)
    .await?;

    println!("[ETAPA 1] BANCO DE DADOS POPULADO E CONECTADO COM SUCESSO");
    println!(
        "----------------------------------------------------------------------------------------------------"
    );

    let epochs = [
        ("Época 0 (t = 0 anos)", Duration::new(0.0)),
        (
            "Época 1 (t = 50 anos)",
            Duration::new(50.0 * 365.25 * 86400.0),
        ),
        (
            "Época 2 (t = 100 anos)",
            Duration::new(100.0 * 365.25 * 86400.0),
        ),
    ];

    println!("[ETAPA 2] RESOLUÇÃO DE POSIÇÕES VIA EPHEMERIS COM PRECESSÃO SECULAR:");
    for (label, t) in &epochs {
        let positions = resolve_system_positions(&pool, system_id, *t).await?;
        let pos_helca = positions.get(&helca_id).copied().unwrap_or_else(Position::zero);
        let pos_p1 = positions.get(&planet1_id).copied().unwrap_or_else(Position::zero);
        let pos_p2 = positions.get(&planet2_id).copied().unwrap_or_else(Position::zero);

        let d_helca_p1 = (pos_p1 - pos_helca).magnitude().value();
        let d_p1_p2 = (pos_p2 - pos_p1).magnitude().value();

        println!("  {}:", label);
        println!("    Posição Hélca  : [{:>12.4e}, {:>12.4e}, {:>12.4e}] m", pos_helca.raw().0, pos_helca.raw().1, pos_helca.raw().2);
        println!("    Posição Zód I  : [{:>12.4e}, {:>12.4e}, {:>12.4e}] m (Dist. Hélca: {:.4} AU)", pos_p1.raw().0, pos_p1.raw().1, pos_p1.raw().2, d_helca_p1 / ASTRONOMICAL_UNIT);
        println!("    Posição Zód II : [{:>12.4e}, {:>12.4e}, {:>12.4e}] m (Separação Interplanetária: {:.4} AU)", pos_p2.raw().0, pos_p2.raw().1, pos_p2.raw().2, d_p1_p2 / ASTRONOMICAL_UNIT);
    }
    println!();

    println!("[ETAPA 3] PONTOS DE LAGRANGE DO PAR HÉLCA - ZÓD I (PROPAGAÇÃO TEMPORAL):");
    println!(
        "----------------------------------------------------------------------------------------------------"
    );
    for (label, t) in &epochs {
        let lagrange_map = resolve_lagrange_points(
            &pool,
            system_id,
            helca_id,
            planet1_id,
            Duration::new(0.0),
            *t,
        )
        .await?;

        let l1 = lagrange_map.get(&astronomicon_core::math::lagrange::LagrangePoint::L1).unwrap();
        let l4 = lagrange_map.get(&astronomicon_core::math::lagrange::LagrangePoint::L4).unwrap();

        println!("  {}:", label);
        println!("    Ponto L1: [{:>12.4e}, {:>12.4e}, {:>12.4e}] m", l1.raw().0, l1.raw().1, l1.raw().2);
        println!("    Ponto L4: [{:>12.4e}, {:>12.4e}, {:>12.4e}] m", l4.raw().0, l4.raw().1, l4.raw().2);
    }
    println!();

    println!("[ETAPA 4] DIAGNÓSTICO DE RESSONÂNCIA ORBITAL ENTRE ZÓD I E ZÓD II:");
    println!(
        "----------------------------------------------------------------------------------------------------"
    );
    let resonance_opt = resolve_orbital_resonance(
        &pool,
        system_id,
        planet1_id,
        planet2_id,
        Duration::new(0.0),
        Duration::new(0.0),
        100,
    )
    .await?;

    if let Some(res) = resonance_opt {
        println!("  Ressonância Detectada : {}:{} (Ordem {})", res.p, res.q, res.resonance_order);
        println!("  Desvio Normalizado    : {:>10.4} %", res.normalized_deviation * 100.0);
        println!("  Estado Dinâmico       : {:?}", res.state);
        println!("  Ângulo Crítico Atual  : {:>10.2}° ({:.4} rad)", res.current_critical_angle.value() * 180.0 / PI, res.current_critical_angle.value());
    }
    println!();

    println!("[ETAPA 5] DIAGNÓSTICOS DE ESTABILIDADE GRAVITACIONAL E ESFERAS DE HILL:");
    println!(
        "----------------------------------------------------------------------------------------------------"
    );
    let stability = resolve_barycenter_stability(&pool, &system_id, &barycenter_id).await?;
    let kozai = resolve_kozai_lidov_diagnostic(&pool, &system_id, &barycenter_id).await?;
    let hill_p1 = resolve_hill_sphere(&pool, &system_id, &planet1_id).await?;
    let hill_p2 = resolve_hill_sphere(&pool, &system_id, &planet2_id).await?;

    let p1_pos = resolve_system_positions(&pool, system_id, Duration::new(0.0)).await?.get(&planet1_id).copied().unwrap();
    let net_acc_p1 = resolve_net_gravitational_acceleration(&pool, &system_id, p1_pos, Duration::new(0.0)).await?;

    println!("  Estabilidade Mardling-Aarseth : Razão Real = {:.4} | Crítica = {:.4} | Estável = {}", stability.actual_ratio, stability.critical_ratio, stability.is_stable);
    println!("  Diagnóstico Kozai-Lidov       : e_max = {:.4} | Ativo = {} | Escala = {:.2} anos", kozai.max_eccentricity, kozai.is_active, kozai.oscillation_timescale.value() / (365.25 * 86400.0));
    println!("  Raio da Esfera de Hill Zód I  : {:>12.4e} m ({:.4} AU)", hill_p1.value(), hill_p1.value() / ASTRONOMICAL_UNIT);
    println!("  Raio da Esfera de Hill Zód II : {:>12.4e} m ({:.4} AU)", hill_p2.value(), hill_p2.value() / ASTRONOMICAL_UNIT);
    println!("  Aceleração Gravitacional em P1: {:>12.6e} m/s²", net_acc_p1.magnitude().value());
    println!();

    assert!(stability.is_stable);
    assert!(hill_p1.value() > 0.0);
    assert!(hill_p2.value() > 0.0);

    println!(
        "===================================================================================================="
    );
    println!(
        "Status do Smoke Test: PIPELINE INTEGRADO DE PONTA A PONTA EXECUTADO COM SUCESSO E CONSISTÊNCIA TOTAL."
    );
    println!(
        "===================================================================================================="
    );

    Ok(())
}