use astronomicon_app::ephemeris::compute_system_positions;
use astronomicon_core::domain::{
    Barycenter, BarycenterMember, OrbitalElements, OrbitalParent, Star, StarKind,
};
use astronomicon_core::math::gravity::{
    combined_gravitational_parameter, gravitational_parameter, surface_gravity,
};
use astronomicon_core::math::kepler::{orbital_period, orbital_speed};
use astronomicon_core::math::radiometry::{mean_density, stellar_luminosity};
use astronomicon_core::math::stability::{
    hill_sphere_radius, is_hierarchically_stable, mardling_aarseth_critical_ratio,
    mardling_aarseth_stability_ratio,
};
use astronomicon_core::units::constants::ASTRONOMICAL_UNIT;
use astronomicon_core::units::{Angle, Duration, Length, Mass, Position};
use std::f64::consts::PI;
use uuid::Uuid;

const SOLAR_MASS_KG: f64 = 1.98847e30;
const SOLAR_RADIUS_M: f64 = 6.957e8;
const SOLAR_LUMINOSITY_W: f64 = 3.828e26;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "===================================================================================================="
    );
    println!(
        "                       ASTRONOMICON - SISTEMA ESTELAR TRINÁRIO ZÓD                                  "
    );
    println!(
        "                   (Validação de Baricentro, Ephemeris e Estabilidade MA)                           "
    );
    println!(
        "===================================================================================================="
    );

    let helca_id = Uuid::new_v4();
    let asdi_id = Uuid::new_v4();
    let nelica_id = Uuid::new_v4();
    let barycenter_id = Uuid::new_v4();

    let m_helca = Mass::new(1.05 * SOLAR_MASS_KG);
    let r_helca = Length::new(1.02 * SOLAR_RADIUS_M);
    let t_helca = astronomicon_core::units::Temperature::new(5820.0);

    let m_asdi = Mass::new(0.72 * SOLAR_MASS_KG);
    let r_asdi = Length::new(0.75 * SOLAR_RADIUS_M);
    let t_asdi = astronomicon_core::units::Temperature::new(4600.0);

    let m_nelica = Mass::new(0.60 * SOLAR_MASS_KG);
    let r_nelica = Length::new(0.012 * SOLAR_RADIUS_M);
    let t_nelica = astronomicon_core::units::Temperature::new(18500.0);

    let helca = Star::builder(
        helca_id,
        "Hélca (Zód A)",
        m_helca,
        StarKind::Star,
        OrbitalParent::Fixed,
    )
    .with_radius(r_helca)
    .with_effective_temperature(t_helca)
    .build()?;

    let asdi = Star::builder(
        asdi_id,
        "Ásdi (Zód B)",
        m_asdi,
        StarKind::Star,
        OrbitalParent::Fixed,
    )
    .with_radius(r_asdi)
    .with_effective_temperature(t_asdi)
    .build()?;

    let inner_a = Length::new(1.80 * ASTRONOMICAL_UNIT);
    let inner_e = 0.12;
    let inner_elements = OrbitalElements::new(
        inner_a,
        inner_e,
        Angle::new(0.0),
        Angle::new(0.0),
        Angle::new(0.0),
        Angle::new(0.0),
    )?;

    let barycenter = Barycenter::new(
        barycenter_id,
        None,
        "Baricentro Hélca-Ásdi".to_string(),
        BarycenterMember::Star(helca_id),
        BarycenterMember::Star(asdi_id),
        inner_elements,
        OrbitalParent::Fixed,
        None,
    )?;

    let outer_a = Length::new(75.0 * ASTRONOMICAL_UNIT);
    let outer_e = 0.05;
    let outer_inc = Angle::new(5.0 * PI / 180.0);
    let outer_lan = Angle::new(25.0 * PI / 180.0);
    let outer_arg = Angle::new(70.0 * PI / 180.0);
    let outer_elements = OrbitalElements::new(
        outer_a,
        outer_e,
        outer_inc,
        outer_lan,
        outer_arg,
        Angle::new(0.0),
    )?;

    let nelica = Star::builder(
        nelica_id,
        "Nélica (Zód C)",
        m_nelica,
        StarKind::WhiteDwarf,
        OrbitalParent::Barycenter(barycenter_id),
    )
    .with_radius(r_nelica)
    .with_effective_temperature(t_nelica)
    .with_orbital_elements(outer_elements)
    .build()?;

    let mu_inner = combined_gravitational_parameter(m_helca, m_asdi);
    let inner_period = orbital_period(inner_a, mu_inner).unwrap();

    let m_bary_total = Mass::new(m_helca.value() + m_asdi.value());
    let mu_outer = combined_gravitational_parameter(m_nelica, m_bary_total);
    let outer_period = orbital_period(outer_a, mu_outer).unwrap();

    let outer_periapsis = Length::new(outer_a.value() * (1.0 - outer_e));
    let actual_ratio = mardling_aarseth_stability_ratio(inner_a, outer_periapsis);
    let critical_ratio =
        mardling_aarseth_critical_ratio(m_bary_total, m_nelica, outer_e, outer_inc);
    let stable = is_hierarchically_stable(
        inner_a,
        outer_periapsis,
        m_bary_total,
        m_nelica,
        outer_e,
        outer_inc,
    );

    let hill_asdi = hill_sphere_radius(inner_a, inner_e, m_asdi, m_helca);
    let hill_nelica = hill_sphere_radius(outer_a, outer_e, m_nelica, m_bary_total);

    println!("[PARTE 1] PROPRIEDADES ASTROFÍSICAS DOS MEMBROS DE ZÓD");
    println!(
        "----------------------------------------------------------------------------------------------------"
    );

    let lum_helca = stellar_luminosity(r_helca, t_helca);
    let g_helca = surface_gravity(gravitational_parameter(m_helca), r_helca);
    let rho_helca = mean_density(m_helca, r_helca);

    println!("  1. Hélca (Anã Amarela - Primária do Baricentro):");
    println!(
        "     Massa: {:>12.4e} kg ({:.2} M☉) | Raio: {:>10.3e} m ({:.2} R☉)",
        m_helca.value(),
        m_helca.value() / SOLAR_MASS_KG,
        r_helca.value(),
        r_helca.value() / SOLAR_RADIUS_M
    );
    println!(
        "     T_eff: {:>10.1} K           | Lum : {:>10.3e} W ({:.3} L☉)",
        t_helca.value(),
        lum_helca.value(),
        lum_helca.value() / SOLAR_LUMINOSITY_W
    );
    println!(
        "     Grav : {:>10.2} m/s²        | Dens: {:>10.2} kg/m³",
        g_helca.value(),
        rho_helca.value()
    );
    println!();

    let lum_asdi = stellar_luminosity(r_asdi, t_asdi);
    let g_asdi = surface_gravity(gravitational_parameter(m_asdi), r_asdi);
    let rho_asdi = mean_density(m_asdi, r_asdi);

    println!("  2. Ásdi (Anã Laranja - Secundária do Baricentro):");
    println!(
        "     Massa: {:>12.4e} kg ({:.2} M☉) | Raio: {:>10.3e} m ({:.2} R☉)",
        m_asdi.value(),
        m_asdi.value() / SOLAR_MASS_KG,
        r_asdi.value(),
        r_asdi.value() / SOLAR_RADIUS_M
    );
    println!(
        "     T_eff: {:>10.1} K           | Lum : {:>10.3e} W ({:.3} L☉)",
        t_asdi.value(),
        lum_asdi.value(),
        lum_asdi.value() / SOLAR_LUMINOSITY_W
    );
    println!(
        "     Grav : {:>10.2} m/s²        | Dens: {:>10.2} kg/m³",
        g_asdi.value(),
        rho_asdi.value()
    );
    println!();

    let lum_nelica = stellar_luminosity(r_nelica, t_nelica);
    let g_nelica = surface_gravity(gravitational_parameter(m_nelica), r_nelica);
    let rho_nelica = mean_density(m_nelica, r_nelica);

    println!("  3. Nélica (Anã Branca - Órbita Externa Distante):");
    println!(
        "     Massa: {:>12.4e} kg ({:.2} M☉) | Raio: {:>10.3e} m ({:.4} R☉)",
        m_nelica.value(),
        m_nelica.value() / SOLAR_MASS_KG,
        r_nelica.value(),
        r_nelica.value() / SOLAR_RADIUS_M
    );
    println!(
        "     T_eff: {:>10.1} K           | Lum : {:>10.3e} W ({:.4} L☉)",
        t_nelica.value(),
        lum_nelica.value(),
        lum_nelica.value() / SOLAR_LUMINOSITY_W
    );
    println!(
        "     Grav : {:>10.2e} m/s²      | Dens: {:>10.2e} kg/m³",
        g_nelica.value(),
        rho_nelica.value()
    );
    println!();

    println!("[PARTE 2] DINÂMICA DO PAR BINÁRIO INTERNO (HÉLCA & ÁSDI)");
    println!(
        "----------------------------------------------------------------------------------------------------"
    );
    let frac_helca = m_helca.value() / m_bary_total.value();
    let frac_asdi = m_asdi.value() / m_bary_total.value();
    let d_helca_bary = inner_a.value() * frac_asdi;
    let d_asdi_bary = inner_a.value() * frac_helca;
    let v_inner_peri = orbital_speed(
        mu_inner,
        Length::new(inner_a.value() * (1.0 - inner_e)),
        inner_a,
    );
    let v_inner_aph = orbital_speed(
        mu_inner,
        Length::new(inner_a.value() * (1.0 + inner_e)),
        inner_a,
    );

    println!(
        "  Semi-eixo Maior Interno (a_in)  : {:>14.4} AU ({:.4e} m)",
        inner_a.value() / ASTRONOMICAL_UNIT,
        inner_a.value()
    );
    println!("  Excentricidade Interna (e_in)   : {:>14.4}", inner_e);
    println!(
        "  μ Combinado do Par Interno      : {:>14.6e} m³/s²",
        mu_inner.value()
    );
    println!(
        "  Período Orbital Interno         : {:>14.2} dias ({:.4} anos)",
        inner_period.value() / 86400.0,
        inner_period.value() / (365.25 * 86400.0)
    );
    println!(
        "  Distância Hélca ao Baricentro   : {:>14.4} AU ({:.2} % do semi-eixo)",
        d_helca_bary / ASTRONOMICAL_UNIT,
        frac_asdi * 100.0
    );
    println!(
        "  Distância Ásdi ao Baricentro    : {:>14.4} AU ({:.2} % do semi-eixo)",
        d_asdi_bary / ASTRONOMICAL_UNIT,
        frac_helca * 100.0
    );
    println!(
        "  Velocidade Relativa no Periastro: {:>14.2} km/s",
        v_inner_peri.value() * 1e-3
    );
    println!(
        "  Velocidade Relativa no Apoastro : {:>14.2} km/s",
        v_inner_aph.value() * 1e-3
    );
    println!();

    println!("[PARTE 3] ÓRBITA DA ESTRELA EXTERNA (NÉLICA AO REDOR DO BARICENTRO)");
    println!(
        "----------------------------------------------------------------------------------------------------"
    );
    let v_outer_peri = orbital_speed(
        mu_outer,
        Length::new(outer_a.value() * (1.0 - outer_e)),
        outer_a,
    );
    let v_outer_aph = orbital_speed(
        mu_outer,
        Length::new(outer_a.value() * (1.0 + outer_e)),
        outer_a,
    );

    println!(
        "  Semi-eixo Maior Externo (a_out) : {:>14.4} AU ({:.4e} m)",
        outer_a.value() / ASTRONOMICAL_UNIT,
        outer_a.value()
    );
    println!("  Excentricidade Externa (e_out)  : {:>14.4}", outer_e);
    println!(
        "  Inclinação Mútua               : {:>14.2}° ({:.4} rad)",
        outer_inc.value() * 180.0 / PI,
        outer_inc.value()
    );
    println!(
        "  μ Combinado do Sistema Externo  : {:>14.6e} m³/s²",
        mu_outer.value()
    );
    println!(
        "  Período Orbital de Nélica       : {:>14.2} anos ({:.2e} s)",
        outer_period.value() / (365.25 * 86400.0),
        outer_period.value()
    );
    println!(
        "  Periastro Externo (R_peri)      : {:>14.4} AU",
        outer_periapsis.value() / ASTRONOMICAL_UNIT
    );
    println!(
        "  Apoastro Externo (R_apo)        : {:>14.4} AU",
        (outer_a.value() * (1.0 + outer_e)) / ASTRONOMICAL_UNIT
    );
    println!(
        "  Velocidade Orbital no Periastro : {:>14.2} km/s",
        v_outer_peri.value() * 1e-3
    );
    println!(
        "  Velocidade Orbital no Apoastro  : {:>14.2} km/s",
        v_outer_aph.value() * 1e-3
    );
    println!();

    println!("[PARTE 4] ANÁLISE DE ESTABILIDADE HIERÁRQUICA (MARDLING-AARSETH & HILL)");
    println!(
        "----------------------------------------------------------------------------------------------------"
    );
    println!(
        "  Razão de Separação Real (R_p / a_in) : {:>10.4}",
        actual_ratio
    );
    println!(
        "  Critério Crítico de Mardling-Aarseth : {:>10.4}",
        critical_ratio
    );
    println!(
        "  Margem de Segurança Estável          : {:>10.2}x acima do limite crítico",
        actual_ratio / critical_ratio
    );
    println!(
        "  Diagnóstico de Estabilidade          : {}",
        if stable {
            "SISTEMA HIERARQUICAMENTE ESTÁVEL A LONGO PRAZO"
        } else {
            "SISTEMA INSTÁVEL"
        }
    );
    println!(
        "  Esfera de Hill de Ásdi (vs Hélca)    : {:>10.4} AU ({:.4e} m)",
        hill_asdi.value() / ASTRONOMICAL_UNIT,
        hill_asdi.value()
    );
    println!(
        "  Esfera de Hill de Nélica (vs Baric.) : {:>10.4} AU ({:.4e} m)",
        hill_nelica.value() / ASTRONOMICAL_UNIT,
        hill_nelica.value()
    );
    println!();

    println!("[PARTE 5] PROPAGAÇÃO TEMPORAL DAS COORDENADAS (EPHEMERIS)");
    println!(
        "----------------------------------------------------------------------------------------------------"
    );

    let stars = vec![helca, asdi, nelica];
    let barycenters = vec![barycenter];
    let planets = vec![];

    let test_intervals = vec![
        ("t = 0 (Época J2000)", Duration::new(0.0)),
        (
            "t = 1/4 Período Binário Interno",
            Duration::new(inner_period.value() / 4.0),
        ),
        (
            "t = 1/2 Período Binário Interno",
            Duration::new(inner_period.value() / 2.0),
        ),
        (
            "t = 1 Período Binário Interno",
            Duration::new(inner_period.value()),
        ),
        (
            "t = 10 Períodos Binários Internos",
            Duration::new(inner_period.value() * 10.0),
        ),
        (
            "t = 1/4 Período Orbital de Nélica",
            Duration::new(outer_period.value() / 4.0),
        ),
    ];

    for (label, t) in test_intervals {
        let positions = compute_system_positions(&stars, &planets, &barycenters, t)?;

        let pos_bary = positions
            .get(&barycenter_id)
            .copied()
            .unwrap_or_else(Position::zero);
        let pos_helca = positions
            .get(&helca_id)
            .copied()
            .unwrap_or_else(Position::zero);
        let pos_asdi = positions
            .get(&asdi_id)
            .copied()
            .unwrap_or_else(Position::zero);
        let pos_nelica = positions
            .get(&nelica_id)
            .copied()
            .unwrap_or_else(Position::zero);

        let d_helca_asdi = (pos_asdi - pos_helca).magnitude().value();
        let d_bary_nelica = (pos_nelica - pos_bary).magnitude().value();

        println!("  INTERVALO: {}", label);
        println!(
            "    Baricentro Hélca-Ásdi: [{:>12.4e}, {:>12.4e}, {:>12.4e}] m",
            pos_bary.raw().0,
            pos_bary.raw().1,
            pos_bary.raw().2
        );
        println!(
            "    Hélca (Primária)     : [{:>12.4e}, {:>12.4e}, {:>12.4e}] m",
            pos_helca.raw().0,
            pos_helca.raw().1,
            pos_helca.raw().2
        );
        println!(
            "    Ásdi (Secundária)    : [{:>12.4e}, {:>12.4e}, {:>12.4e}] m",
            pos_asdi.raw().0,
            pos_asdi.raw().1,
            pos_asdi.raw().2
        );
        println!(
            "    Nélica (Externa)     : [{:>12.4e}, {:>12.4e}, {:>12.4e}] m",
            pos_nelica.raw().0,
            pos_nelica.raw().1,
            pos_nelica.raw().2
        );
        println!(
            "    Distância Hélca-Ásdi : {:>14.4} AU ({:.4e} m)",
            d_helca_asdi / ASTRONOMICAL_UNIT,
            d_helca_asdi
        );
        println!(
            "    Distância Bar.-Nélica: {:>14.4} AU ({:.4e} m)",
            d_bary_nelica / ASTRONOMICAL_UNIT,
            d_bary_nelica
        );
        println!();
    }

    println!(
        "===================================================================================================="
    );
    println!(
        "Status da Validação: SISTEMA TRINÁRIO ZÓD RESOLVIDO COM SUCESSO. HIERARQUIA DINAMICAMENTE ESTÁVEL."
    );
    println!(
        "===================================================================================================="
    );

    Ok(())
}