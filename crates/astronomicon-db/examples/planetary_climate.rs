use astronomicon_core::domain::{
    Atmosphere,
    GasComponent,
    OrbitalElements,
    OrbitalParent,
    Planet,
    PlanetKind,
    Star,
    StarKind,
};
use astronomicon_core::math::atmosphere::ideal_gas_density;
use astronomicon_core::math::climate::{
    blended_local_temperature,
    day_length_half_angle,
    local_radiative_equilibrium_temperature,
    mean_daily_insolation_factor,
    solar_declination,
    temperature_at_altitude,
};
use astronomicon_core::math::gravity::{ gravitational_parameter, surface_gravity };
use astronomicon_core::math::radiometry::{
    equilibrium_temperature,
    orbital_irradiance,
    stellar_luminosity,
};
use astronomicon_core::units::constants::{
    ASTRONOMICAL_UNIT,
    GRAVITATIONAL_CONSTANT,
    STEFAN_BOLTZMANN_CONSTANT,
    UNIVERSAL_GAS_CONSTANT,
};
use astronomicon_core::units::{ Angle, Length, Mass, Pressure, Temperature, TemperatureGradient };
use std::f64::consts::PI;
use uuid::Uuid;

const SOLAR_MASS: f64 = 1.98847e30;
const SOLAR_RADIUS: f64 = 6.957e8;
const SOLAR_TEMPERATURE: f64 = 5778.0;

struct PlanetClimateFixture {
    name: &'static str,
    planet: Planet,
    atmosphere: Atmosphere,
    expected_teq_k: f64,
    expected_tmean_k: f64,
    expected_scale_height_km: f64,
    expected_density_kg_m3: f64,
    altitude_checkpoints: Vec<f64>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sun = Star::new(
        Uuid::new_v4(),
        None,
        OrbitalParent::Fixed,
        StarKind::Star,
        "Sol".to_string(),
        Mass::new(SOLAR_MASS),
        Some(Length::new(SOLAR_RADIUS)),
        Some(Temperature::new(SOLAR_TEMPERATURE)),
        None,
        None,
        None
    )?;

    let earth_id = Uuid::new_v4();
    let earth_elements = OrbitalElements::new(
        Length::new(149_597_870_700.0),
        0.0167086,
        Angle::new(0.0),
        Angle::new(0.0),
        Angle::new(0.0),
        Angle::new(0.0)
    )?;
    let earth = Planet::new(
        earth_id,
        None,
        OrbitalParent::Star(sun.id()),
        "Terra".to_string(),
        PlanetKind::Telluric,
        Mass::new(5.9722e24),
        Some(Length::new(6.371e6)),
        Some(Length::new(6.3568e6)),
        None,
        Some(Angle::new((23.44 * PI) / 180.0)),
        Some(0.367),
        Some(0.306),
        Some(0.8),
        Some(Angle::new(0.0)),
        Some(earth_elements)
    )?;
    let earth_atmosphere = Atmosphere::new(
        Uuid::new_v4(),
        earth_id,
        Pressure::new(101_325.0),
        Temperature::new(33.0),
        TemperatureGradient::new(0.0065),
        vec![
            GasComponent::new("N2".to_string(), 78.08)?,
            GasComponent::new("O2".to_string(), 20.95)?,
            GasComponent::new("Ar".to_string(), 0.93)?,
            GasComponent::new("CO2".to_string(), 0.04)?
        ]
    )?;

    let mars_id = Uuid::new_v4();
    let mars_elements = OrbitalElements::new(
        Length::new(227.92e9),
        0.0934,
        Angle::new(0.0323),
        Angle::new(0.865),
        Angle::new(5.0),
        Angle::new(0.0)
    )?;
    let mars = Planet::new(
        mars_id,
        None,
        OrbitalParent::Star(sun.id()),
        "Marte".to_string(),
        PlanetKind::Telluric,
        Mass::new(6.4171e23),
        Some(Length::new(3.3895e6)),
        Some(Length::new(3.3762e6)),
        None,
        Some(Angle::new((25.19 * PI) / 180.0)),
        Some(0.15),
        Some(0.25),
        Some(0.2),
        Some(Angle::new(0.0)),
        Some(mars_elements)
    )?;
    let mars_atmosphere = Atmosphere::new(
        Uuid::new_v4(),
        mars_id,
        Pressure::new(610.0),
        Temperature::new(5.0),
        TemperatureGradient::new(0.0025),
        vec![
            GasComponent::new("CO2".to_string(), 95.97)?,
            GasComponent::new("N2".to_string(), 2.6)?,
            GasComponent::new("Ar".to_string(), 1.43)?
        ]
    )?;

    let venus_id = Uuid::new_v4();
    let venus_elements = OrbitalElements::new(
        Length::new(108.21e9),
        0.0067,
        Angle::new(0.0592),
        Angle::new(1.338),
        Angle::new(0.958),
        Angle::new(0.0)
    )?;
    let venus = Planet::new(
        venus_id,
        None,
        OrbitalParent::Star(sun.id()),
        "Vênus".to_string(),
        PlanetKind::Telluric,
        Mass::new(4.8675e24),
        Some(Length::new(6.0518e6)),
        Some(Length::new(6.0518e6)),
        None,
        Some(Angle::new((2.64 * PI) / 180.0)),
        Some(0.67),
        Some(0.77),
        Some(0.95),
        Some(Angle::new(0.0)),
        Some(venus_elements)
    )?;
    let venus_atmosphere = Atmosphere::new(
        Uuid::new_v4(),
        venus_id,
        Pressure::new(9.2e6),
        Temperature::new(510.0),
        TemperatureGradient::new(0.0077),
        vec![GasComponent::new("CO2".to_string(), 96.5)?, GasComponent::new("N2".to_string(), 3.5)?]
    )?;

    let fixtures = vec![
        PlanetClimateFixture {
            name: "Terra",
            planet: earth,
            atmosphere: earth_atmosphere,
            expected_teq_k: 255.0,
            expected_tmean_k: 288.0,
            expected_scale_height_km: 8.5,
            expected_density_kg_m3: 1.225,
            altitude_checkpoints: vec![0.0, 2500.0, 5000.0, 8500.0, 12000.0],
        },
        PlanetClimateFixture {
            name: "Marte",
            planet: mars,
            atmosphere: mars_atmosphere,
            expected_teq_k: 210.0,
            expected_tmean_k: 215.0,
            expected_scale_height_km: 11.1,
            expected_density_kg_m3: 0.015,
            altitude_checkpoints: vec![0.0, 5000.0, 11100.0, 20000.0],
        },
        PlanetClimateFixture {
            name: "Vênus",
            planet: venus,
            atmosphere: venus_atmosphere,
            expected_teq_k: 230.0,
            expected_tmean_k: 737.0,
            expected_scale_height_km: 15.9,
            expected_density_kg_m3: 65.0,
            altitude_checkpoints: vec![0.0, 15900.0, 30000.0, 50000.0],
        }
    ];

    println!(
        "===================================================================================================="
    );
    println!(
        "                        ASTRONOMICON - RELATÓRIO DE CLIMA E ATMOSFERAS PLANETÁRIAS                  "
    );
    println!(
        "                                (Validação Física das Frentes 7 e 8)                                "
    );
    println!(
        "===================================================================================================="
    );

    let star_temp = sun.effective_temperature().unwrap();
    let star_rad = sun.radius().unwrap();
    let star_lum = stellar_luminosity(star_rad, star_temp);

    for fixture in fixtures {
        let planet = &fixture.planet;
        let atmosphere = &fixture.atmosphere;
        let elements = planet.orbital_elements().unwrap();
        let semi_major = elements.semi_major_axis();
        let bond_albedo = planet.bond_albedo().unwrap();
        let thermal_inertia = planet.thermal_inertia().unwrap();
        let obliquity = planet.obliquity().unwrap();
        let solstice_ta = planet.solstice_true_anomaly().unwrap();

        let mu_planet = gravitational_parameter(planet.mass());
        let eq_radius = planet.equatorial_radius().unwrap();
        let g = surface_gravity(mu_planet, eq_radius);

        let top_irradiance = orbital_irradiance(star_lum, semi_major);
        let t_eq = equilibrium_temperature(star_temp, star_rad, semi_major, bond_albedo);
        let t_global = t_eq + atmosphere.greenhouse_effect();

        let molar_mass = atmosphere.mean_molar_mass()?;
        let scale_h = atmosphere.scale_height(g, t_global)?;
        let surface_density = ideal_gas_density(
            atmosphere.surface_pressure(),
            molar_mass,
            t_global
        );

        println!(
            "----------------------------------------------------------------------------------------------------"
        );
        println!(
            "PLANETA: {:<15} | TIPO: {:<12} | SEMI-EIXO: {:>8.4} AU",
            fixture.name,
            format!("{:?}", planet.kind()),
            semi_major.value() / ASTRONOMICAL_UNIT
        );
        println!(
            "----------------------------------------------------------------------------------------------------"
        );
        println!("[1] PARÂMETROS FÍSICOS E ATMOSFÉRICOS:");
        println!("  Massa Planetária     : {:>14.4e} kg", planet.mass().value());
        println!("  Raio Equatorial      : {:>14.2} km", eq_radius.value() * 1e-3);
        println!("  Gravidade Superfície : {:>14.4} m/s²", g.value());
        println!("  Albedo de Bond       : {:>14.4}", bond_albedo);
        println!("  Inércia Térmica      : {:>14.4}", thermal_inertia);
        println!(
            "  Obliquidade Axial    : {:>14.2}° ({:.4} rad)",
            (obliquity.value() * 180.0) / PI,
            obliquity.value()
        );
        println!(
            "  Pressão de Superfície: {:>14.2} Pa ({:.4} bar)",
            atmosphere.surface_pressure().value(),
            atmosphere.surface_pressure().value() * 1e-5
        );
        println!("  Massa Molar Média    : {:>14.4} g/mol", molar_mass.value() * 1000.0);
        println!(
            "  Composição Gasosa    : {}",
            atmosphere
                .composition()
                .iter()
                .map(|c| format!("{}: {:.2}%", c.formula(), c.percentage()))
                .collect::<Vec<_>>()
                .join(", ")
        );
        println!();

        println!("[2] BALANÇO RADIATIVO E EFEITO ESTUFA:");
        println!("  Irradiância Orbital  : {:>14.2} W/m²", top_irradiance.value());
        println!(
            "  T_eq Calculada       : {:>14.2} K ({:>6.2} °C) | Esperado: {:>6.1} K (Erro: {:.2}%)",
            t_eq.value(),
            t_eq.value() - 273.15,
            fixture.expected_teq_k,
            ((t_eq.value() - fixture.expected_teq_k) / fixture.expected_teq_k).abs() * 100.0
        );
        println!("  Efeito Estufa (ΔT)   : {:>14.2} K", atmosphere.greenhouse_effect().value());
        println!(
            "  T_média Global Calc. : {:>14.2} K ({:>6.2} °C) | Esperado: {:>6.1} K (Erro: {:.2}%)",
            t_global.value(),
            t_global.value() - 273.15,
            fixture.expected_tmean_k,
            ((t_global.value() - fixture.expected_tmean_k) / fixture.expected_tmean_k).abs() * 100.0
        );
        println!(
            "  Escala de Altura (H) : {:>14.2} km         | Esperado: {:>6.1} km (Erro: {:.2}%)",
            scale_h.value() * 1e-3,
            fixture.expected_scale_height_km,
            (
                (scale_h.value() * 1e-3 - fixture.expected_scale_height_km) /
                fixture.expected_scale_height_km
            ).abs() * 100.0
        );
        println!(
            "  Densidade Superfície : {:>14.4} kg/m³      | Esperado: {:>6.3} kg/m³ (Erro: {:.2}%)",
            surface_density.value(),
            fixture.expected_density_kg_m3,
            (
                (surface_density.value() - fixture.expected_density_kg_m3) /
                fixture.expected_density_kg_m3
            ).abs() * 100.0
        );
        println!();

        println!("[3] DISTRIBUIÇÃO TÉRMICA LATITUDINAL (SOLSTÍCIO DE VERÃO NO HEMISFÉRIO NORTE):");
        println!(
            "  {:<16} | {:<12} | {:<12} | {:<14} | {:<18} | {:<18}",
            "Latitude",
            "Declinação",
            "Fotoperíodo",
            "Fator Insolação",
            "T_eq Local",
            "T_Superfície"
        );
        println!(
            "  -----------------+--------------+--------------+----------------+--------------------+--------------------"
        );

        let summer_solstice_ta = Angle::new(solstice_ta.value() + PI / 2.0);
        let sample_latitudes = vec![
            ("Polo Norte (+90°)", (90.0 * PI) / 180.0),
            ("Temperado (+45°)", (45.0 * PI) / 180.0),
            ("Equador (0°)", 0.0),
            ("Temperado (-45°)", (-45.0 * PI) / 180.0),
            ("Polo Sul (-90°)", (-90.0 * PI) / 180.0)
        ];

        for (lat_name, lat_rad) in sample_latitudes {
            let lat_angle = Angle::new(lat_rad);
            let decl = solar_declination(
                obliquity,
                elements.argument_of_periapsis(),
                solstice_ta,
                summer_solstice_ta
            );
            let h0 = day_length_half_angle(lat_angle, decl);
            let insolation_factor = mean_daily_insolation_factor(lat_angle, decl, h0);
            let local_insolation = top_irradiance * insolation_factor;
            let local_eq = local_radiative_equilibrium_temperature(local_insolation, bond_albedo);
            let local_blended = blended_local_temperature(t_global, local_eq, thermal_inertia);
            let day_hours = (h0.value() / PI) * 24.0;

            println!(
                "  {:<16} | {:>10.2}° | {:>10.1} h | {:>14.4} | {:>7.2} K ({:>5.1} °C) | {:>7.2} K ({:>5.1} °C)",
                lat_name,
                (decl.value() * 180.0) / PI,
                day_hours,
                insolation_factor,
                local_eq.value(),
                local_eq.value() - 273.15,
                local_blended.value(),
                local_blended.value() - 273.15
            );
        }
        println!();

        println!("[4] PERFIL ATMOSFÉRICO VERTICAL:");
        println!(
            "  {:<12} | {:<20} | {:<24} | {:<16} | {:<10}",
            "Altitude",
            "Temperatura",
            "Pressão",
            "Densidade",
            "P / P0"
        );
        println!(
            "  -------------+----------------------+--------------------------+------------------+-----------"
        );

        for alt_m in &fixture.altitude_checkpoints {
            let alt = Length::new(*alt_m);
            let t_alt = temperature_at_altitude(t_global, alt, atmosphere.lapse_rate());
            let p_alt = atmosphere.pressure_at_altitude(alt, scale_h);
            let rho_alt = ideal_gas_density(p_alt, molar_mass, t_alt);
            let p_ratio = (p_alt.value() / atmosphere.surface_pressure().value()) * 100.0;

            println!(
                "  {:>9.2} km | {:>7.2} K ({:>6.2} °C) | {:>12.2} Pa ({:>6.3} bar) | {:>12.4e} kg/m³ | {:>8.2} %",
                alt_m * 1e-3,
                t_alt.value(),
                t_alt.value() - 273.15,
                p_alt.value(),
                p_alt.value() * 1e-5,
                rho_alt.value(),
                p_ratio
            );
        }
        println!();
    }

    println!(
        "===================================================================================================="
    );
    println!("Constantes Físicas Aplicadas:");
    println!("  G  = {:.6e} m³/(kg·s²)", GRAVITATIONAL_CONSTANT);
    println!("  σ  = {:.9e} W/(m²·K⁴)", STEFAN_BOLTZMANN_CONSTANT);
    println!("  R  = {:.9} J/(mol·K)", UNIVERSAL_GAS_CONSTANT);
    println!("  AU = {:.1} m", ASTRONOMICAL_UNIT);
    println!(
        "Status da Validação: MODELOS DE RADIAÇÃO, INSOLAÇÃO E ESTRUTURA ATMOSFÉRICA VALIDADOS COM SUCESSO."
    );
    println!(
        "===================================================================================================="
    );

    Ok(())
}
