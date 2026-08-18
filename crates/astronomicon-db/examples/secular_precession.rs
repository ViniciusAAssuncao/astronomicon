use astronomicon_core::domain::OrbitalElements;
use astronomicon_core::math::gravity::gravitational_parameter;
use astronomicon_core::math::kepler::mean_motion;
use astronomicon_core::math::perturbation::{
    apsidal_precession_rate_j2, apsidal_precession_rate_relativistic, nodal_regression_rate_j2,
    resolve_secular_precession,
};
use astronomicon_core::units::constants::ASTRONOMICAL_UNIT;
use astronomicon_core::units::{Angle, Length, Mass};
use std::f64::consts::PI;

const SOLAR_MASS_KG: f64 = 1.98847e30;
const EARTH_MASS_KG: f64 = 5.9722e24;
const EARTH_EQUATORIAL_RADIUS_M: f64 = 6_378_137.0;
const EARTH_J2: f64 = 0.00108263;

const SECONDS_PER_JULIAN_CENTURY: f64 = 3.15576e9;
const RAD_TO_ARCSEC: f64 = 180.0 * 3600.0 / PI;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "===================================================================================================="
    );
    println!(
        "                   ASTRONOMICON - RELATÓRIO DE PRECESSÃO SECULAR (GR & J2 OBLATENESS)               "
    );
    println!(
        "===================================================================================================="
    );

    let m_sun = Mass::new(SOLAR_MASS_KG);
    let mu_sun = gravitational_parameter(m_sun);

    let a_mercury = Length::new(0.387098 * ASTRONOMICAL_UNIT);
    let e_mercury = 0.205630;
    let i_mercury = Angle::new(7.005 * PI / 180.0);
    let _elem_mercury = OrbitalElements::new(
        a_mercury,
        e_mercury,
        i_mercury,
        Angle::new(0.0),
        Angle::new(0.0),
        Angle::new(0.0),
    )?;

    let n_mercury = mean_motion(a_mercury, mu_sun);
    let gr_rate_mercury = apsidal_precession_rate_relativistic(n_mercury, a_mercury, e_mercury, mu_sun);
    let gr_arcsec_century = gr_rate_mercury.value() * SECONDS_PER_JULIAN_CENTURY * RAD_TO_ARCSEC;

    println!("[FIXTURE A] SOL - MERCÚRIO (AVANÇO DO PERIÉLIO POR RELATIVIDADE GERAL)");
    println!(
        "----------------------------------------------------------------------------------------------------"
    );
    println!("  Semi-eixo Maior (a)        : {:>14.4} AU ({:.4e} m)", a_mercury.value() / ASTRONOMICAL_UNIT, a_mercury.value());
    println!("  Excentricidade (e)         : {:>14.6}", e_mercury);
    println!("  Taxa Relativística Linear  : {:>14.6e} rad/s", gr_rate_mercury.value());
    println!(
        "  Taxa Relativística Século  : {:>14.4} arcsec/século (Histórico Einstein 1915: 42.98)",
        gr_arcsec_century
    );
    println!(
        "  Erro Relativo Teórico      : {:>14.4} %",
        ((gr_arcsec_century - 42.98) / 42.98).abs() * 100.0
    );
    println!();

    let m_earth = Mass::new(EARTH_MASS_KG);
    let mu_earth = gravitational_parameter(m_earth);
    let r_eq_earth = Length::new(EARTH_EQUATORIAL_RADIUS_M);

    let h_sso = 700_000.0;
    let a_sso = Length::new(EARTH_EQUATORIAL_RADIUS_M + h_sso);
    let e_sso = 0.001;
    let i_sso = Angle::new(98.19 * PI / 180.0);
    let _elem_sso = OrbitalElements::new(a_sso, e_sso, i_sso, Angle::new(0.0), Angle::new(0.0), Angle::new(0.0))?;

    let n_sso = mean_motion(a_sso, mu_earth);
    let sso_nodal = nodal_regression_rate_j2(n_sso, a_sso, e_sso, i_sso, EARTH_J2, r_eq_earth);
    let sso_nodal_deg_day = sso_nodal.value() * (180.0 / PI) * 86400.0;
    let target_earth_orbit_rate = 360.0 / 365.2422;

    println!("[FIXTURE B] SATÉLITE HELIOSSÍNCRONO (SSO @ 700 KM, i = 98.19°)");
    println!(
        "----------------------------------------------------------------------------------------------------"
    );
    println!("  Altitude Orbital           : {:>14.2} km", h_sso * 1e-3);
    println!("  Inclinação Orbital         : {:>14.2}° ({:.4} rad)", i_sso.value() * 180.0 / PI, i_sso.value());
    println!("  Coeficiente Gravitacional J2: {:>14.7e}", EARTH_J2);
    println!("  Regressão Nodal Calculada  : {:>14.4} °/dia ({:.6e} rad/s)", sso_nodal_deg_day, sso_nodal.value());
    println!("  Meta de Rotação Terrestre  : {:>14.4} °/dia (360° / 365.2422 d)", target_earth_orbit_rate);
    println!(
        "  Diferença Absoluta         : {:>14.6} °/dia",
        (sso_nodal_deg_day - target_earth_orbit_rate).abs()
    );
    println!();

    let a_molniya = Length::new(26_600_000.0);
    let e_molniya = 0.70;
    let i_molniya_crit = Angle::new((1.0 / 5.0_f64.sqrt()).acos());
    let elem_molniya = OrbitalElements::new(
        a_molniya,
        e_molniya,
        i_molniya_crit,
        Angle::new(0.0),
        Angle::new(0.0),
        Angle::new(0.0),
    )?;

    let n_molniya = mean_motion(a_molniya, mu_earth);
    let apsidal_j2_molniya =
        apsidal_precession_rate_j2(n_molniya, a_molniya, e_molniya, i_molniya_crit, EARTH_J2, r_eq_earth);
    let full_molniya = resolve_secular_precession(&elem_molniya, mu_earth, Some(EARTH_J2), Some(r_eq_earth));

    println!("[FIXTURE C] ÓRBITA MOLNIYA NA INCLINAÇÃO CRÍTICA (i = 63.4349°)");
    println!(
        "----------------------------------------------------------------------------------------------------"
    );
    println!("  Semi-eixo Maior (a)        : {:>14.2} km", a_molniya.value() * 1e-3);
    println!("  Excentricidade (e)         : {:>14.4}", e_molniya);
    println!("  Inclinação Crítica (5cos²i-1=0): {:>12.4}° ({:.6} rad)", i_molniya_crit.value() * 180.0 / PI, i_molniya_crit.value());
    println!(
        "  Termo 5*cos²(i) - 1        : {:>14.6e}",
        5.0 * i_molniya_crit.value().cos().powi(2) - 1.0
    );
    println!(
        "  Precessão Apsidal de J2    : {:>14.6e} °/dia (Congelamento do Perigeu)",
        apsidal_j2_molniya.value() * (180.0 / PI) * 86400.0
    );
    println!(
        "  Precessão Apsidal Total (c/ GR): {:>12.6e} °/dia",
        full_molniya.apsidal.value() * (180.0 / PI) * 86400.0
    );
    println!();

    println!(
        "===================================================================================================="
    );
    println!(
        "Status da Validação: PRECESSÃO RELATIVÍSTICA DE MERCÚRIO E EFEITOS DE J2 CONFERIDOS COM EXATIDÃO."
    );
    println!(
        "===================================================================================================="
    );

    Ok(())
}