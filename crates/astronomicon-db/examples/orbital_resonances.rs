use astronomicon_core::domain::OrbitalElements;
use astronomicon_core::math::gravity::combined_gravitational_parameter;
use astronomicon_core::math::kepler::{mean_longitude_at_epoch, mean_motion, orbital_period};
use astronomicon_core::math::resonance::{
    classify_libration, laplace_resonant_argument, mean_motion_resonance_search, resonance_order,
    resonant_argument, ResonanceState,
};
use astronomicon_core::units::constants::ASTRONOMICAL_UNIT;
use astronomicon_core::units::{Angle, Duration, Length, Mass};
use std::f64::consts::PI;

const SOLAR_MASS_KG: f64 = 1.98847e30;
const JUPITER_MASS_KG: f64 = 1.89813e27;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "===================================================================================================="
    );
    println!(
        "                      ASTRONOMICON - RELATÓRIO DE RESSONÂNCIAS ORBITAIS E FRAÇÕES CONTÍNUAS         "
    );
    println!(
        "===================================================================================================="
    );

    let m_sun = Mass::new(SOLAR_MASS_KG);
    let m_neptune = Mass::new(1.02413e26);
    let m_pluto = Mass::new(1.303e22);

    let a_neptune = Length::new(30.06992 * ASTRONOMICAL_UNIT);
    let e_neptune = 0.008678;
    let i_neptune = Angle::new(1.769 * PI / 180.0);
    let lan_neptune = Angle::new(131.78 * PI / 180.0);
    let arg_neptune = Angle::new(273.187 * PI / 180.0);
    let m0_neptune = Angle::new(256.228 * PI / 180.0);

    let elem_neptune = OrbitalElements::new(
        a_neptune,
        e_neptune,
        i_neptune,
        lan_neptune,
        arg_neptune,
        m0_neptune,
    )?;

    let a_pluto = Length::new(39.482 * ASTRONOMICAL_UNIT);
    let e_pluto = 0.2488;
    let i_pluto = Angle::new(17.16 * PI / 180.0);
    let lan_pluto = Angle::new(110.299 * PI / 180.0);
    let arg_pluto = Angle::new(113.834 * PI / 180.0);
    let m0_pluto = Angle::new(14.53 * PI / 180.0);

    let elem_pluto = OrbitalElements::new(a_pluto, e_pluto, i_pluto, lan_pluto, arg_pluto, m0_pluto)?;

    let mu_neptune = combined_gravitational_parameter(m_sun, m_neptune);
    let mu_pluto = combined_gravitational_parameter(m_sun, m_pluto);

    let n_neptune = mean_motion(a_neptune, mu_neptune);
    let n_pluto = mean_motion(a_pluto, mu_pluto);

    let t_neptune = orbital_period(a_neptune, mu_neptune).unwrap();
    let t_pluto = orbital_period(a_pluto, mu_pluto).unwrap();

    let search_np = mean_motion_resonance_search(n_neptune, n_pluto, 32);

    println!("[FIXTURE A] NETUNO - PLUTÃO (RESSONÂNCIA 3:2)");
    println!(
        "----------------------------------------------------------------------------------------------------"
    );
    println!("  Período Orbital Netuno     : {:>12.2} anos ({:.2} s)", t_neptune.value() / (365.25 * 86400.0), t_neptune.value());
    println!("  Período Orbital Plutão     : {:>12.2} anos ({:.2} s)", t_pluto.value() / (365.25 * 86400.0), t_pluto.value());
    println!("  Movimento Médio Netuno (n1): {:>12.6e} rad/s", n_neptune.value());
    println!("  Movimento Médio Plutão (n2): {:>12.6e} rad/s", n_pluto.value());
    println!("  Razão Observada n1 / n2    : {:>12.6}", n_neptune.value() / n_pluto.value());

    if let Some((p, q, dev)) = search_np {
        let order = resonance_order(p, q);
        println!(
            "  Ressonância Identificada   : {}:{} (Ordem {})",
            p, q, order
        );
        println!("  Desvio Normalizado         : {:>12.6e} %", dev * 100.0);

        let delta_n = (n_neptune.value() - n_pluto.value()).abs();
        let synodic_period = 2.0 * PI / delta_n;
        let time_span = synodic_period * (p as f64);
        let samples = 100;

        let mut angles = Vec::with_capacity(samples);
        for i in 0..samples {
            let t = Duration::new((i as f64 / (samples - 1) as f64) * time_span);
            let lambda_nep = mean_longitude_at_epoch(&elem_neptune, n_neptune, t);
            let lambda_plu = mean_longitude_at_epoch(&elem_pluto, n_pluto, t);
            let varpi_plu = elem_pluto.longitude_of_periapsis();

            let phi = resonant_argument(p, q, lambda_nep, lambda_plu, varpi_plu);
            angles.push(phi);
        }

        let state = classify_libration(&angles);
        println!("  Período Sinódico           : {:>12.2} anos", synodic_period / (365.25 * 86400.0));
        println!("  Ângulo Crítico Inicial (t0): {:>12.2}° ({:.4} rad)", angles[0].value() * 180.0 / PI, angles[0].value());
        println!("  Classificação Dinâmica     : {:?}", state);
    }
    println!();

    let m_jup = Mass::new(JUPITER_MASS_KG);
    let m_io = Mass::new(8.9319e22);
    let m_europa = Mass::new(4.7998e22);
    let m_ganymede = Mass::new(1.4819e23);

    let a_io = Length::new(421.7e6);
    let a_europa = Length::new(670.9e6);
    let a_ganymede = Length::new(1070.4e6);

    let elem_io = OrbitalElements::new(a_io, 0.0041, Angle::new(0.0), Angle::new(0.0), Angle::new(0.0), Angle::new(PI))?;
    let elem_europa = OrbitalElements::new(a_europa, 0.009, Angle::new(0.0), Angle::new(0.0), Angle::new(0.0), Angle::new(0.0))?;
    let elem_ganymede = OrbitalElements::new(a_ganymede, 0.0013, Angle::new(0.0), Angle::new(0.0), Angle::new(0.0), Angle::new(0.0))?;

    let mu_io = combined_gravitational_parameter(m_jup, m_io);
    let mu_europa = combined_gravitational_parameter(m_jup, m_europa);
    let mu_ganymede = combined_gravitational_parameter(m_jup, m_ganymede);

    let n_io = mean_motion(a_io, mu_io);
    let n_europa = mean_motion(a_europa, mu_europa);
    let n_ganymede = mean_motion(a_ganymede, mu_ganymede);

    let t_io = orbital_period(a_io, mu_io).unwrap();
    let t_europa = orbital_period(a_europa, mu_europa).unwrap();
    let t_ganymede = orbital_period(a_ganymede, mu_ganymede).unwrap();

    let search_io_eur = mean_motion_resonance_search(n_io, n_europa, 10).unwrap();
    let search_eur_gan = mean_motion_resonance_search(n_europa, n_ganymede, 10).unwrap();

    println!("[FIXTURE B] CADEIA DE LAPLACE (IO - EUROPA - GANIMEDES: 4:2:1)");
    println!(
        "----------------------------------------------------------------------------------------------------"
    );
    println!("  Período Io (T1)            : {:>10.4} dias ({:.4e} rad/s)", t_io.value() / 86400.0, n_io.value());
    println!("  Período Europa (T2)        : {:>10.4} dias ({:.4e} rad/s)", t_europa.value() / 86400.0, n_europa.value());
    println!("  Período Ganimedes (T3)     : {:>10.4} dias ({:.4e} rad/s)", t_ganymede.value() / 86400.0, n_ganymede.value());
    println!(
        "  MMR Io - Europa            : {}:{} (Desvio: {:.4} %)",
        search_io_eur.0, search_io_eur.1, search_io_eur.2 * 100.0
    );
    println!(
        "  MMR Europa - Ganimedes     : {}:{} (Desvio: {:.4} %)",
        search_eur_gan.0, search_eur_gan.1, search_eur_gan.2 * 100.0
    );

    let sample_count = 100;
    let laplace_span = t_ganymede.value() * 4.0;
    let mut laplace_angles = Vec::with_capacity(sample_count);

    for i in 0..sample_count {
        let t = Duration::new((i as f64 / (sample_count - 1) as f64) * laplace_span);
        let l1 = mean_longitude_at_epoch(&elem_io, n_io, t);
        let l2 = mean_longitude_at_epoch(&elem_europa, n_europa, t);
        let l3 = mean_longitude_at_epoch(&elem_ganymede, n_ganymede, t);

        let phi_l = laplace_resonant_argument(l1, l2, l3);
        laplace_angles.push(phi_l);
    }

    let laplace_state = classify_libration(&laplace_angles);
    println!("  Ângulo de Laplace Φ_L (t0) : {:>10.2}° ({:.4} rad)", laplace_angles[0].value() * 180.0 / PI, laplace_angles[0].value());
    println!("  Classificação Cadeia       : {:?}", laplace_state);
    println!();

    let a_earth = Length::new(1.0 * ASTRONOMICAL_UNIT);
    let a_jup_ext = Length::new(5.2044 * ASTRONOMICAL_UNIT);
    let elem_earth = OrbitalElements::new(a_earth, 0.0167, Angle::new(0.0), Angle::new(0.0), Angle::new(0.0), Angle::new(0.0))?;
    let elem_jup_ext = OrbitalElements::new(a_jup_ext, 0.0484, Angle::new(0.0227), Angle::new(1.75), Angle::new(4.77), Angle::new(0.34))?;

    let n_earth = mean_motion(a_earth, combined_gravitational_parameter(m_sun, Mass::new(5.9722e24)));
    let n_jup_ext = mean_motion(a_jup_ext, combined_gravitational_parameter(m_sun, m_jup));

    println!("[FIXTURE C] CONTROLE NEGATIVO (TERRA - JÚPITER: NÃO-RESSONANTE)");
    println!(
        "----------------------------------------------------------------------------------------------------"
    );
    let search_ej = mean_motion_resonance_search(n_earth, n_jup_ext, 32);
    if let Some((p, q, dev)) = search_ej {
        println!("  Melhor Aproximação         : {}:{} (Ordem {})", p, q, resonance_order(p, q));
        println!("  Desvio da Fração           : {:>10.4} %", dev * 100.0);

        let delta_n_res = (p as f64 * n_jup_ext.value() - q as f64 * n_earth.value()).abs();
        let circulation_period = if delta_n_res > 1e-15 {
            2.0 * PI / delta_n_res
        } else {
            2.0 * PI / (n_earth.value() - n_jup_ext.value()).abs()
        };
        let time_span = circulation_period * 1.5;
        let mut ej_angles = Vec::with_capacity(100);

        for i in 0..100 {
            let t = Duration::new((i as f64 / 99.0) * time_span);
            let l_e = mean_longitude_at_epoch(&elem_earth, n_earth, t);
            let l_j = mean_longitude_at_epoch(&elem_jup_ext, n_jup_ext, t);
            let varpi_j = elem_jup_ext.longitude_of_periapsis();
            let phi = resonant_argument(p, q, l_e, l_j, varpi_j);
            ej_angles.push(phi);
        }

        let state_ej = classify_libration(&ej_angles);
        println!("  Período de Circulação      : {:>10.2} anos", circulation_period / (365.25 * 86400.0));
        println!("  Classificação Dinâmica     : {:?}", state_ej);
        assert_eq!(state_ej, ResonanceState::Circulating);
    }
    println!();

    println!(
        "===================================================================================================="
    );
    println!(
        "Nota Física: A propagação Kepleriana pura a 2 corpos identifica a comensurabilidade cinemática nos"
    );
    println!(
        "elementos osculadores atuais. Ela reproduz a libração aparente em janelas de poucos períodos sinódicos,"
    );
    println!(
        "mas não simula a retroalimentação n-corpos de captura gravitacional a longo prazo."
    );
    println!(
        "===================================================================================================="
    );

    Ok(())
}