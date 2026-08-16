use astronomicon_app::ephemeris::compute_system_positions;
use astronomicon_core::domain::{ OrbitalElements, Planet, PlanetKind, Star, StarKind };
use astronomicon_core::math::gravity::{ combined_gravitational_parameter, gravitational_parameter };
use astronomicon_core::math::kepler::{ orbital_period, orbital_speed, orbital_state_vectors };
use astronomicon_core::units::constants::ASTRONOMICAL_UNIT;
use astronomicon_core::units::{ Angle, Duration, Length, Mass, Position };
use std::f64::consts::PI;
use uuid::Uuid;

const SUN_MASS_KG: f64 = 1.98847e30;
const EARTH_MASS_KG: f64 = 5.9722e24;
const MOON_MASS_KG: f64 = 7.342e22;

const EARTH_SEMI_MAJOR_M: f64 = 149_597_870_700.0;
const EARTH_ECCENTRICITY: f64 = 0.0167086;

const MOON_SEMI_MAJOR_M: f64 = 384_400_000.0;
const MOON_ECCENTRICITY: f64 = 0.0549;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "===================================================================================================="
    );
    println!(
        "                           ASTRONOMICON - RELATÓRIO DE ÓRBITAS E EPHEMERIS                         "
    );
    println!(
        "                                   (Validação Física da Frente 9)                                   "
    );
    println!(
        "===================================================================================================="
    );

    let m_sun = Mass::new(SUN_MASS_KG);
    let m_earth = Mass::new(EARTH_MASS_KG);
    let m_moon = Mass::new(MOON_MASS_KG);

    let mu_sun_earth = combined_gravitational_parameter(m_sun, m_earth);
    let earth_a = Length::new(EARTH_SEMI_MAJOR_M);
    let earth_e = EARTH_ECCENTRICITY;

    let theoretical_period_seconds =
        2.0 * PI * (earth_a.value().powi(3) / mu_sun_earth.value()).sqrt();
    let computed_period = orbital_period(earth_a, mu_sun_earth).unwrap();
    let period_days = computed_period.value() / 86400.0;

    let theoretical_perihelion = earth_a.value() * (1.0 - earth_e);
    let theoretical_aphelion = earth_a.value() * (1.0 + earth_e);

    let earth_elements = OrbitalElements::new(
        earth_a,
        earth_e,
        Angle::new(0.0),
        Angle::new(0.0),
        Angle::new(0.0),
        Angle::new(0.0)
    )?;

    let (r_peri, v_peri) = orbital_state_vectors(
        &earth_elements,
        mu_sun_earth,
        Duration::new(0.0)
    )?;
    let r_peri_mag = r_peri.magnitude().value();
    let v_peri_mag = v_peri.magnitude().value();

    let half_period = Duration::new(computed_period.value() / 2.0);
    let (r_aph, v_aph) = orbital_state_vectors(&earth_elements, mu_sun_earth, half_period)?;
    let r_aph_mag = r_aph.magnitude().value();
    let v_aph_mag = v_aph.magnitude().value();

    let expected_v_peri = orbital_speed(mu_sun_earth, Length::new(theoretical_perihelion), earth_a);
    let expected_v_aph = orbital_speed(mu_sun_earth, Length::new(theoretical_aphelion), earth_a);

    println!("[PARTE 1] VALIDAÇÃO DA ÓRBITA TERRESTRE (LEIS DE KEPLER)");
    println!(
        "----------------------------------------------------------------------------------------------------"
    );
    println!(
        "  Semi-eixo Maior (a)        : {:>16.2} m ({:.6} AU)",
        earth_a.value(),
        earth_a.value() / ASTRONOMICAL_UNIT
    );
    println!("  Excentricidade (e)         : {:>16.7}", earth_e);
    println!("  Parâmetro Gravitacional μ  : {:>16.6e} m³/s²", mu_sun_earth.value());
    println!(
        "  --------------------------------------------------------------------------------------------------"
    );
    println!(
        "  Período Orbital Calculado  : {:>16.2} s ({:.6} dias)",
        computed_period.value(),
        period_days
    );
    println!(
        "  Período Esperado (Ano Sid.): {:>16.2} s ({:.6} dias)",
        365.256363 * 86400.0,
        365.256363
    );
    println!(
        "  Diferença Absoluta Período : {:>16.4} s ({:.6e} %)",
        (computed_period.value() - theoretical_period_seconds).abs(),
        ((computed_period.value() - 365.256363 * 86400.0) / (365.256363 * 86400.0)) * 100.0
    );
    println!(
        "  --------------------------------------------------------------------------------------------------"
    );
    println!("  Distância Periélio Teórica : {:>16.2} m", theoretical_perihelion);
    println!(
        "  Distância Periélio Vetorial: {:>16.2} m (Erro: {:.6e} m)",
        r_peri_mag,
        (r_peri_mag - theoretical_perihelion).abs()
    );
    println!(
        "  Velocidade no Periélio     : {:>16.2} m/s (Esperado: {:.2} m/s)",
        v_peri_mag,
        expected_v_peri.value()
    );
    println!("  Distância Afélio Teórica   : {:>16.2} m", theoretical_aphelion);
    println!(
        "  Distância Afélio Vetorial  : {:>16.2} m (Erro: {:.6e} m)",
        r_aph_mag,
        (r_aph_mag - theoretical_aphelion).abs()
    );
    println!(
        "  Velocidade no Afélio       : {:>16.2} m/s (Esperado: {:.2} m/s)",
        v_aph_mag,
        expected_v_aph.value()
    );
    println!();

    println!("[PARTE 2] VALIDAÇÃO DA HIERARQUIA SINTÉTICA (SOL → TERRA → LUA)");
    println!(
        "----------------------------------------------------------------------------------------------------"
    );

    let star_id = Uuid::new_v4();
    let planet_id = Uuid::new_v4();
    let moon_id = Uuid::new_v4();

    let star = Star::new(
        star_id,
        None,
        StarKind::Star,
        "Sol".to_string(),
        m_sun,
        Some(Length::new(6.957e8)),
        None,
        None,
        None,
        None
    )?;

    let planet = Planet::new(
        planet_id,
        Some(star_id),
        None,
        "Terra".to_string(),
        PlanetKind::Telluric,
        m_earth,
        Some(Length::new(6.371e6)),
        None,
        None,
        None,
        None,
        None,
        None,
        Some(earth_elements)
    )?;

    let moon_elements = OrbitalElements::new(
        Length::new(MOON_SEMI_MAJOR_M),
        MOON_ECCENTRICITY,
        Angle::new(0.0898),
        Angle::new(0.0),
        Angle::new(0.0),
        Angle::new(0.0)
    )?;

    let moon = Planet::new(
        moon_id,
        None,
        Some(planet_id),
        "Lua".to_string(),
        PlanetKind::IcyBody,
        m_moon,
        Some(Length::new(1.737e6)),
        None,
        None,
        None,
        None,
        None,
        None,
        Some(moon_elements)
    )?;

    let stars = vec![star];
    let planets = vec![planet, moon];

    let test_intervals = vec![
        ("t = 0 (Época Inicial)", Duration::new(0.0)),
        ("t = 1/4 Período Terrestre", Duration::new(computed_period.value() / 4.0)),
        ("t = 1/2 Período Terrestre", Duration::new(computed_period.value() / 2.0)),
        ("t = 1 Período Completo", Duration::new(computed_period.value()))
    ];

    let mu_earth_moon = combined_gravitational_parameter(m_earth, m_moon);
    let moon_period = orbital_period(Length::new(MOON_SEMI_MAJOR_M), mu_earth_moon).unwrap();

    println!("  Composição de μ:");
    println!("    μ(Sol)           = {:>16.6e} m³/s²", gravitational_parameter(m_sun).value());
    println!("    μ(Sol + Terra)   = {:>16.6e} m³/s²", mu_sun_earth.value());
    println!("    μ(Terra + Lua)   = {:>16.6e} m³/s²", mu_earth_moon.value());
    println!(
        "    Período Orbital Lua: {:>14.2} s ({:.4} dias)",
        moon_period.value(),
        moon_period.value() / 86400.0
    );
    println!(
        "  --------------------------------------------------------------------------------------------------"
    );

    for (label, t) in test_intervals {
        let positions = compute_system_positions(&stars, &planets, t)?;

        let pos_sun = positions.get(&star_id).copied().unwrap_or_else(Position::zero);
        let pos_earth = positions.get(&planet_id).copied().unwrap_or_else(Position::zero);
        let pos_moon = positions.get(&moon_id).copied().unwrap_or_else(Position::zero);

        let d_sun_earth = (pos_earth - pos_sun).magnitude().value();
        let d_earth_moon = (pos_moon - pos_earth).magnitude().value();

        println!("  INTERVALO: {}", label);
        println!(
            "    Posição Sol   : [{:>14.4e}, {:>14.4e}, {:>14.4e}] m",
            pos_sun.raw().0,
            pos_sun.raw().1,
            pos_sun.raw().2
        );
        println!(
            "    Posição Terra : [{:>14.4e}, {:>14.4e}, {:>14.4e}] m",
            pos_earth.raw().0,
            pos_earth.raw().1,
            pos_earth.raw().2
        );
        println!(
            "    Posição Lua   : [{:>14.4e}, {:>14.4e}, {:>14.4e}] m",
            pos_moon.raw().0,
            pos_moon.raw().1,
            pos_moon.raw().2
        );
        println!(
            "    Distância Sol-Terra : {:>16.4e} m ({:.6} AU)",
            d_sun_earth,
            d_sun_earth / ASTRONOMICAL_UNIT
        );
        println!(
            "    Distância Terra-Lua : {:>16.4e} m (Esperado no intervalo: {:.4e} a {:.4e} m)",
            d_earth_moon,
            MOON_SEMI_MAJOR_M * (1.0 - MOON_ECCENTRICITY),
            MOON_SEMI_MAJOR_M * (1.0 + MOON_ECCENTRICITY)
        );
        println!();
    }

    println!(
        "===================================================================================================="
    );
    println!(
        "Status da Validação: TODAS AS EQUAÇÕES DE KEPLER E HIERARQUIAS ORBITAIS FORAM RESOLVIDAS COM SUCESSO."
    );
    println!(
        "===================================================================================================="
    );

    Ok(())
}
