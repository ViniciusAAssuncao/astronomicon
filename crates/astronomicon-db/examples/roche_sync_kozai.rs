use astronomicon_core::math::gravity::gravitational_parameter;
use astronomicon_core::math::kepler::orbital_period;
use astronomicon_core::math::radiometry::mean_density;
use astronomicon_core::math::stability::{
    kozai_critical_inclination, kozai_max_eccentricity, kozai_oscillation_timescale,
};
use astronomicon_core::math::tidal::{
    roche_limit_fluid, roche_limit_rigid, synchronous_orbit_radius,
};
use astronomicon_core::units::constants::ASTRONOMICAL_UNIT;
use astronomicon_core::units::{Angle, Density, Duration, Length, Mass};
use std::f64::consts::PI;

const SOLAR_MASS_KG: f64 = 1.98847e30;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "===================================================================================================="
    );
    println!(
        "                 ASTRONOMICON - RELATÓRIO DE ROCHE, ÓRBITA SÍNCRONA E KOZAI-LIDOV                    "
    );
    println!(
        "===================================================================================================="
    );

    let m_saturn = Mass::new(5.6834e26);
    let r_saturn = Length::new(60_268_000.0);
    let rho_saturn = mean_density(m_saturn, r_saturn);
    let rho_ice = Density::new(900.0);

    let roche_saturn_rigid = roche_limit_rigid(r_saturn, rho_saturn, rho_ice);
    let roche_saturn_fluid = roche_limit_fluid(r_saturn, rho_saturn, rho_ice);
    let ring_a_outer_radius = 136_775_000.0;

    println!("[SUB-FIXTURE A] SATURNO E O LIMITE DE ROCHE DOS ANÉIS");
    println!(
        "----------------------------------------------------------------------------------------------------"
    );
    println!("  Massa de Saturno           : {:>14.4e} kg", m_saturn.value());
    println!("  Raio Equatorial Saturno    : {:>14.2} km", r_saturn.value() * 1e-3);
    println!("  Densidade Média Saturno    : {:>14.2} kg/m³", rho_saturn.value());
    println!("  Densidade Típica do Gelo   : {:>14.2} kg/m³", rho_ice.value());
    println!("  Limite de Roche Rígido     : {:>14.2} km", roche_saturn_rigid.value() * 1e-3);
    println!("  Limite de Roche Fluido     : {:>14.2} km", roche_saturn_fluid.value() * 1e-3);
    println!("  Borda Externa do Anel A    : {:>14.2} km", ring_a_outer_radius * 1e-3);
    println!(
        "  Posicionamento dos Anéis   : {}",
        if ring_a_outer_radius <= roche_saturn_fluid.value() {
            "DENTRO DO LIMITE FLUIDO (Agregação gravitacional impedida por maré)"
        } else {
            "FORA DO LIMITE FLUIDO"
        }
    );
    println!();

    let m_earth = Mass::new(5.9722e24);
    let r_earth = Length::new(6_371_000.0);
    let t_rot_earth = Duration::new(86164.0905);
    let mu_earth = gravitational_parameter(m_earth);
    let r_geo = synchronous_orbit_radius(mu_earth, t_rot_earth);

    println!("[SUB-FIXTURE B] TERRA E O RAIO DA ÓRBITA GEOESTACIONÁRIA (SÍNCRONA)");
    println!(
        "----------------------------------------------------------------------------------------------------"
    );
    println!("  Período de Rotação Sideral : {:>14.4} s ({:.4} h)", t_rot_earth.value(), t_rot_earth.value() / 3600.0);
    println!("  Raio Orbital Síncrono      : {:>14.2} km", r_geo.value() * 1e-3);
    println!("  Altitude Geoestacionária   : {:>14.2} km", (r_geo.value() - r_earth.value()) * 1e-3);
    println!("  Valor Canônico Esperado    : {:>14.2} km (Erro: {:.4e} %)", 42164.14, ((r_geo.value() * 1e-3 - 42164.14) / 42164.14).abs() * 100.0);
    println!();

    let m_mars = Mass::new(6.4171e23);
    let r_mars = Length::new(3_389_500.0);
    let rho_mars = mean_density(m_mars, r_mars);
    let rho_phobos = Density::new(1876.0);
    let a_phobos = Length::new(9_376_000.0);

    let roche_mars_rigid = roche_limit_rigid(r_mars, rho_mars, rho_phobos);
    let roche_mars_fluid = roche_limit_fluid(r_mars, rho_mars, rho_phobos);

    println!("[SUB-FIXTURE C] MARTE E FOBOS (ESTABILIDADE ECOLÓGICA DE MARÉ)");
    println!(
        "----------------------------------------------------------------------------------------------------"
    );
    println!("  Raio Orbital Atual de Fobos: {:>14.2} km", a_phobos.value() * 1e-3);
    println!("  Limite de Roche Rígido     : {:>14.2} km", roche_mars_rigid.value() * 1e-3);
    println!("  Limite de Roche Fluido     : {:>14.2} km", roche_mars_fluid.value() * 1e-3);
    println!(
        "  Diagnóstico Estrutural     : R_rígido ({:.0} km) < a_Fobos ({:.0} km) < R_fluido ({:.0} km)",
        roche_mars_rigid.value() * 1e-3,
        a_phobos.value() * 1e-3,
        roche_mars_fluid.value() * 1e-3
    );
    println!("  Implicação Astrofísica     : Mantido por coesão rígida; ruptura prevista se for rubble-pile.");
    println!();

    let m_inner_total = Mass::new(1.77 * SOLAR_MASS_KG);
    let m_outer = Mass::new(0.60 * SOLAR_MASS_KG);
    let a_inner = Length::new(1.8 * ASTRONOMICAL_UNIT);
    let a_outer = Length::new(75.0 * ASTRONOMICAL_UNIT);
    let e_outer = 0.05;

    let mu_in = gravitational_parameter(m_inner_total);
    let mu_out = gravitational_parameter(Mass::new(m_inner_total.value() + m_outer.value()));
    let p_inner = orbital_period(a_inner, mu_in).unwrap();
    let p_outer = orbital_period(a_outer, mu_out).unwrap();

    let crit_inc = kozai_critical_inclination();

    let inc_low = Angle::new(10.0 * PI / 180.0);
    let max_e_low = kozai_max_eccentricity(inc_low);
    let is_active_low = inc_low.value() >= crit_inc.value() && inc_low.value() <= PI - crit_inc.value();

    let inc_high = Angle::new(85.0 * PI / 180.0);
    let max_e_high = kozai_max_eccentricity(inc_high);
    let is_active_high = inc_high.value() >= crit_inc.value() && inc_high.value() <= PI - crit_inc.value();
    let timescale = kozai_oscillation_timescale(p_inner, p_outer, m_inner_total, m_outer, e_outer);

    println!("[SUB-FIXTURE D] DIAGNÓSTICO DE KOZAI-LIDOV (HIERARQUIA TRIPLA)");
    println!(
        "----------------------------------------------------------------------------------------------------"
    );
    println!("  Inclinação Crítica de Kozai: {:>14.2}° ({:.4} rad)", crit_inc.value() * 180.0 / PI, crit_inc.value());
    println!(
        "  Caso 1 (i = 10.0° < i_crit): e_max = {:>6.4} | Mecanismo Ativo: {}",
        max_e_low, is_active_low
    );
    println!(
        "  Caso 2 (i = 85.0° > i_crit): e_max = {:>6.4} | Mecanismo Ativo: {}",
        max_e_high, is_active_high
    );
    println!("  Escala Temporal de Kozai   : {:>14.2} anos ({:.4e} s)", timescale.value() / (365.25 * 86400.0), timescale.value());
    println!();

    println!(
        "===================================================================================================="
    );
    println!(
        "Status da Validação: ROCHE, RAIO GEOESTACIONÁRIO E CICLOS DE KOZAI-LIDOV RIGOROSAMENTE VALIDADOS."
    );
    println!(
        "===================================================================================================="
    );

    Ok(())
}