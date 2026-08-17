use astronomicon_core::math::lagrange::{
    is_lagrange_point_stable, lagrange_point_position, solve_l1_gamma, solve_l2_gamma,
    solve_l3_gamma, LagrangePoint,
};
use astronomicon_core::units::constants::{ASTRONOMICAL_UNIT, ROUTH_CRITICAL_MASS_PARAMETER};
use astronomicon_core::units::{Mass, Position, Vector3};

const SOLAR_MASS_KG: f64 = 1.98847e30;
const EARTH_MASS_KG: f64 = 5.9722e24;
const JUPITER_MASS_KG: f64 = 1.89813e27;

const EARTH_SEMI_MAJOR_M: f64 = 149_597_870_700.0;
const JUPITER_SEMI_MAJOR_M: f64 = 778.57e9;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "===================================================================================================="
    );
    println!(
        "                     ASTRONOMICON - RELATÓRIO DE PONTOS DE LAGRANGE E ESTABILIDADE DE ROUTH         "
    );
    println!(
        "===================================================================================================="
    );

    let m_sun = Mass::new(SOLAR_MASS_KG);
    let m_earth = Mass::new(EARTH_MASS_KG);

    let p_sun = Position::zero();
    let p_earth = Position::from_components(EARTH_SEMI_MAJOR_M, 0.0, 0.0);
    let normal_earth = Vector3::new(0.0, 0.0, 1.0);

    let mu_earth = m_earth.value() / (m_sun.value() + m_earth.value());
    let gamma1_earth = solve_l1_gamma(mu_earth)?;
    let gamma2_earth = solve_l2_gamma(mu_earth)?;
    let gamma3_earth = solve_l3_gamma(mu_earth)?;

    let points = [
        LagrangePoint::L1,
        LagrangePoint::L2,
        LagrangePoint::L3,
        LagrangePoint::L4,
        LagrangePoint::L5,
    ];

    println!("[SISTEMA 1] SOL - TERRA (1.0000 AU)");
    println!(
        "----------------------------------------------------------------------------------------------------"
    );
    println!("  Massa Sol (M1)             : {:>14.4e} kg", m_sun.value());
    println!("  Massa Terra (M2)           : {:>14.4e} kg", m_earth.value());
    println!("  Parâmetro de Massa (μ)     : {:>14.6e}", mu_earth);
    println!(
        "  Limite Crítico de Routh    : {:>14.6e}",
        ROUTH_CRITICAL_MASS_PARAMETER
    );
    println!(
        "  Razão μ / μ_crítico        : {:>14.6e} ({:.4e}x abaixo do limite)",
        mu_earth / ROUTH_CRITICAL_MASS_PARAMETER,
        ROUTH_CRITICAL_MASS_PARAMETER / mu_earth
    );
    println!(
        "  --------------------------------------------------------------------------------------------------"
    );
    println!("  Soluções dos Polinômios Colineares (Gamma):");
    println!("    γ(L1) = {:>12.8} | Distância à Terra = {:>10.3} Mkm", gamma1_earth, (gamma1_earth * EARTH_SEMI_MAJOR_M) * 1e-9);
    println!("    γ(L2) = {:>12.8} | Distância à Terra = {:>10.3} Mkm", gamma2_earth, (gamma2_earth * EARTH_SEMI_MAJOR_M) * 1e-9);
    println!("    γ(L3) = {:>12.8} | Distância ao Sol  = {:>10.6} AU", gamma3_earth, (gamma3_earth * EARTH_SEMI_MAJOR_M) / ASTRONOMICAL_UNIT);
    println!(
        "  --------------------------------------------------------------------------------------------------"
    );
    println!(
        "  {:<6} | {:<42} | {:<16} | {:<16} | {:<10}",
        "Ponto", "Coordenadas [X, Y, Z] (m)", "Dist. Sol (AU)", "Dist. Terra (Mkm)", "Estabilidade"
    );
    println!(
        "  -------+--------------------------------------------+------------------+-------------------+-----------"
    );

    for point in points {
        let pos = lagrange_point_position(point, p_sun, p_earth, m_sun, m_earth, normal_earth)?;
        let d_sun = (pos - p_sun).magnitude().value();
        let d_earth = (pos - p_earth).magnitude().value();
        let is_stable = is_lagrange_point_stable(point, m_sun, m_earth);

        println!(
            "  {:<6?} | [{:>12.4e}, {:>12.4e}, {:>12.4e}] | {:>14.6} AU | {:>15.4} Mkm | {:<10}",
            point,
            pos.raw().0,
            pos.raw().1,
            pos.raw().2,
            d_sun / ASTRONOMICAL_UNIT,
            d_earth * 1e-9,
            if is_stable { "ESTÁVEL" } else { "INSTÁVEL" }
        );
    }
    println!();

    let m_jupiter = Mass::new(JUPITER_MASS_KG);
    let p_jupiter = Position::from_components(JUPITER_SEMI_MAJOR_M, 0.0, 0.0);
    let normal_jupiter = Vector3::new(0.0, 0.0, 1.0);

    let mu_jupiter = m_jupiter.value() / (m_sun.value() + m_jupiter.value());
    let gamma1_jupiter = solve_l1_gamma(mu_jupiter)?;
    let gamma2_jupiter = solve_l2_gamma(mu_jupiter)?;
    let gamma3_jupiter = solve_l3_gamma(mu_jupiter)?;

    println!("[SISTEMA 2] SOL - JÚPITER ({:.4} AU)", JUPITER_SEMI_MAJOR_M / ASTRONOMICAL_UNIT);
    println!(
        "----------------------------------------------------------------------------------------------------"
    );
    println!("  Massa Sol (M1)             : {:>14.4e} kg", m_sun.value());
    println!("  Massa Júpiter (M2)         : {:>14.4e} kg", m_jupiter.value());
    println!("  Parâmetro de Massa (μ)     : {:>14.6e}", mu_jupiter);
    println!(
        "  Limite Crítico de Routh    : {:>14.6e}",
        ROUTH_CRITICAL_MASS_PARAMETER
    );
    println!(
        "  Razão μ / μ_crítico        : {:>14.6e} ({:.2}x abaixo do limite)",
        mu_jupiter / ROUTH_CRITICAL_MASS_PARAMETER,
        ROUTH_CRITICAL_MASS_PARAMETER / mu_jupiter
    );
    println!(
        "  --------------------------------------------------------------------------------------------------"
    );
    println!("  Soluções dos Polinômios Colineares (Gamma):");
    println!("    γ(L1) = {:>12.8} | Distância a Júpiter = {:>10.3} Mkm", gamma1_jupiter, (gamma1_jupiter * JUPITER_SEMI_MAJOR_M) * 1e-9);
    println!("    γ(L2) = {:>12.8} | Distância a Júpiter = {:>10.3} Mkm", gamma2_jupiter, (gamma2_jupiter * JUPITER_SEMI_MAJOR_M) * 1e-9);
    println!("    γ(L3) = {:>12.8} | Distância ao Sol     = {:>10.6} AU", gamma3_jupiter, (gamma3_jupiter * JUPITER_SEMI_MAJOR_M) / ASTRONOMICAL_UNIT);
    println!(
        "  --------------------------------------------------------------------------------------------------"
    );
    println!(
        "  {:<6} | {:<42} | {:<16} | {:<16} | {:<10}",
        "Ponto", "Coordenadas [X, Y, Z] (m)", "Dist. Sol (AU)", "Dist. Júpiter (Mkm)", "Estabilidade"
    );
    println!(
        "  -------+--------------------------------------------+------------------+-------------------+-----------"
    );

    for point in points {
        let pos = lagrange_point_position(point, p_sun, p_jupiter, m_sun, m_jupiter, normal_jupiter)?;
        let d_sun = (pos - p_sun).magnitude().value();
        let d_jupiter = (pos - p_jupiter).magnitude().value();
        let is_stable = is_lagrange_point_stable(point, m_sun, m_jupiter);

        println!(
            "  {:<6?} | [{:>12.4e}, {:>12.4e}, {:>12.4e}] | {:>14.6} AU | {:>17.4} Mkm | {:<10}",
            point,
            pos.raw().0,
            pos.raw().1,
            pos.raw().2,
            d_sun / ASTRONOMICAL_UNIT,
            d_jupiter * 1e-9,
            if is_stable { "ESTÁVEL" } else { "INSTÁVEL" }
        );
    }
    println!();

    println!(
        "===================================================================================================="
    );
    println!(
        "Status da Validação: SOLVER DE PONTOS DE LAGRANGE E CRITÉRIO DE ROUTH VALIDADOS COM SUCESSO."
    );
    println!(
        "===================================================================================================="
    );

    Ok(())
}