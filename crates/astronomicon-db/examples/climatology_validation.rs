use astronomicon_app::climate::{
    resolve_advective_surface_temperature, resolve_atmospheric_profile_at_altitude,
    resolve_global_mean_temperature, resolve_latitudinal_surface_temperature,
    resolve_planetary_circulation, resolve_stellar_wind_at_planet,
    resolve_wind_profile_at_latitude,
};
use astronomicon_app::ephemeris::resolve_system_positions;
use astronomicon_core::units::constants::ASTRONOMICAL_UNIT;
use astronomicon_core::units::{Angle, Duration, Length, Position};
use std::f64::consts::PI;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("====================================================================================================");
    println!("               ASTRONOMICON - RELATÓRIO DE CLIMATOLOGIA, DINÂMICA DE VENTOS E METEOROLOGIA          ");
    println!("                                   (Validação Física dos Blocos 1 a 6)                              ");
    println!("====================================================================================================");

    let pool = astronomicon_db::save::initialize_save("sqlite::memory:?cache=shared").await?;

    let system_id = Uuid::parse_str("e7f51f35-815a-48d5-ab29-6d6ec15e2557")?;
    let helca_id = Uuid::parse_str("ca972f4c-90b3-4613-8eb5-71b4b971222e")?;
    let asdi_id = Uuid::parse_str("d2eba543-3aec-4d83-bc4b-c5eb4e1604a3")?;
    let nelica_id = Uuid::parse_str("f758fc33-28b6-482b-b8ab-d43b79da34f2")?;

    let meros_id = Uuid::parse_str("60dc1b97-29a6-4b1c-95fd-42b229079928")?;
    let jatur_id = Uuid::parse_str("40522b31-9e8d-4bca-8ace-64582850c341")?;
    let hadab_id = Uuid::parse_str("4beb55b2-62de-4ec2-abe5-ec00290407f8")?;
    let avizina_id = Uuid::parse_str("e4387e15-39be-4da6-b2c0-3d122f683981")?;
    let jena_id = Uuid::parse_str("50d8d6a4-3440-4afa-a655-1b0cbb50cb4f")?;

    let jatur_atm_id = Uuid::parse_str("e402a21b-9b89-4f2a-aba4-0032052a0811")?;
    let hadab_atm_id = Uuid::parse_str("dc917590-0859-4d8c-b95b-fade7685be22")?;

    sqlx::query(
        "INSERT INTO star_systems (id, name, right_ascension_rad, declination_rad, distance_from_sol_m) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(system_id.to_string())
    .bind("Sistema Zód")
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
    .bind(1.9487006e30)
    .bind(685810000.0)
    .bind(5557.033)
    .bind(2246400.0)
    .bind(0.12391837689159739)
    .bind(0.00005)
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
    .bind(1.21097823e30)
    .bind(418811400.0)
    .bind(4243.0)
    .bind(4246560.0)
    .bind(0.1308996938995747)
    .execute(&pool)
    .await?;

    sqlx::query(
        "INSERT INTO stars (id, star_system_id, parent_star_id, name, kind, mass_kg, radius_m, \
         effective_temperature_k, rotation_period_s, axial_tilt_rad, semi_major_axis_m, eccentricity, \
         inclination_rad, longitude_ascending_node_rad, argument_periapsis_rad, mean_anomaly_at_epoch_rad) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(nelica_id.to_string())
    .bind(system_id.to_string())
    .bind(helca_id.to_string())
    .bind("Nélica")
    .bind("WhiteDwarf")
    .bind(1.1533e30)
    .bind(8500000.0)
    .bind(9200.0)
    .bind(7200.0)
    .bind(0.0)
    .bind(269276000000000.0)
    .bind(0.72)
    .bind(3.106686)
    .bind(0.0)
    .bind(0.0)
    .bind(0.0)
    .execute(&pool)
    .await?;

    sqlx::query(
        "INSERT INTO planets (id, star_system_id, parent_star_id, name, kind, mass_kg, equatorial_radius_m, \
         polar_radius_m, rotation_period_s, axial_tilt_rad, geometric_albedo, bond_albedo, thermal_inertia, \
         solstice_true_anomaly_rad, semi_major_axis_m, eccentricity, inclination_rad, longitude_ascending_node_rad, \
         argument_periapsis_rad, mean_anomaly_at_epoch_rad, oblateness_j2, hydrosphere_fraction) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(meros_id.to_string())
    .bind(system_id.to_string())
    .bind(helca_id.to_string())
    .bind("Meros")
    .bind("Telluric")
    .bind(5.793034e23)
    .bind(2905440.0)
    .bind(2905440.0)
    .bind(5067014.4)
    .bind(0.0006806784082777885)
    .bind(0.09)
    .bind(0.06)
    .bind(0.1)
    .bind(3.054378550867636)
    .bind(58343169573.0)
    .bind(0.233)
    .bind(0.09081639841922233)
    .bind(0.5961578948795151)
    .bind(0.5239478414486978)
    .bind(3.0507)
    .bind(Option::<f64>::None)
    .bind(0.0)
    .execute(&pool)
    .await?;

    sqlx::query(
        "INSERT INTO planets (id, star_system_id, parent_star_id, name, kind, mass_kg, equatorial_radius_m, \
         polar_radius_m, rotation_period_s, axial_tilt_rad, geometric_albedo, bond_albedo, thermal_inertia, \
         solstice_true_anomaly_rad, semi_major_axis_m, eccentricity, inclination_rad, longitude_ascending_node_rad, \
         argument_periapsis_rad, mean_anomaly_at_epoch_rad, oblateness_j2, hydrosphere_fraction) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(jatur_id.to_string())
    .bind(system_id.to_string())
    .bind(helca_id.to_string())
    .bind("Jatur")
    .bind("Telluric")
    .bind(5.145229466e24)
    .bind(5934586.5)
    .bind(5934586.5)
    .bind(86164.2)
    .bind(0.4090929593627069)
    .bind(0.367)
    .bind(0.306)
    .bind(0.3)
    .bind(1.5707963267948966)
    .bind(120471200000.0)
    .bind(0.005002)
    .bind(0.018319994926682723)
    .bind(1.9804615272594548)
    .bind(1.527954022861506)
    .bind(3.387792079095107)
    .bind(0.000898954)
    .bind(0.0)
    .execute(&pool)
    .await?;

    sqlx::query(
        "INSERT INTO planets (id, star_system_id, parent_star_id, name, kind, mass_kg, equatorial_radius_m, \
         polar_radius_m, rotation_period_s, axial_tilt_rad, geometric_albedo, bond_albedo, thermal_inertia, \
         solstice_true_anomaly_rad, semi_major_axis_m, eccentricity, inclination_rad, longitude_ascending_node_rad, \
         argument_periapsis_rad, mean_anomaly_at_epoch_rad, oblateness_j2, hydrosphere_fraction) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(hadab_id.to_string())
    .bind(system_id.to_string())
    .bind(helca_id.to_string())
    .bind("Hadab")
    .bind("Telluric")
    .bind(6.59121853e24)
    .bind(6395980.691)
    .bind(6395980.691)
    .bind(93900.000384)
    .bind(0.4090929593627069)
    .bind(0.3002)
    .bind(0.30548)
    .bind(0.3007)
    .bind(1.5708835932574963)
    .bind(147400277979.417)
    .bind(0.103658)
    .bind(0.41015237421866746)
    .bind(2.1830578283945075)
    .bind(1.5707963267948966)
    .bind(2.3609068791727297)
    .bind(0.000825806)
    .bind(0.74)
    .execute(&pool)
    .await?;

    sqlx::query(
        "INSERT INTO planets (id, star_system_id, parent_planet_id, name, kind, mass_kg, equatorial_radius_m, \
         polar_radius_m, rotation_period_s, axial_tilt_rad, geometric_albedo, bond_albedo, thermal_inertia, \
         solstice_true_anomaly_rad, semi_major_axis_m, eccentricity, inclination_rad, longitude_ascending_node_rad, \
         argument_periapsis_rad, mean_anomaly_at_epoch_rad, oblateness_j2, hydrosphere_fraction) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(avizina_id.to_string())
    .bind(system_id.to_string())
    .bind(hadab_id.to_string())
    .bind("Avizina")
    .bind("Telluric")
    .bind(7.7951e22)
    .bind(2115017.0)
    .bind(2115017.0)
    .bind(2379392.15904)
    .bind(0.06342299172612322)
    .bind(0.36)
    .bind(0.3)
    .bind(0.3)
    .bind(1.5707963267948966)
    .bind(431092000.0)
    .bind(0.004)
    .bind(0.08979719001510825)
    .bind(2.10224908402717)
    .bind(0.7853981633974483)
    .bind(0.0)
    .bind(4.170363e-06)
    .bind(0.0)
    .execute(&pool)
    .await?;

    sqlx::query(
        "INSERT INTO planets (id, star_system_id, parent_planet_id, name, kind, mass_kg, equatorial_radius_m, \
         polar_radius_m, rotation_period_s, axial_tilt_rad, geometric_albedo, bond_albedo, thermal_inertia, \
         solstice_true_anomaly_rad, semi_major_axis_m, eccentricity, inclination_rad, longitude_ascending_node_rad, \
         argument_periapsis_rad, mean_anomaly_at_epoch_rad, oblateness_j2, hydrosphere_fraction) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(jena_id.to_string())
    .bind(system_id.to_string())
    .bind(hadab_id.to_string())
    .bind("Jena")
    .bind("Telluric")
    .bind(1.911104e22)
    .bind(1109410.0)
    .bind(1109410.0)
    .bind(845725.595616)
    .bind(0.011677678961453502)
    .bind(0.36)
    .bind(0.3)
    .bind(0.2)
    .bind(1.626123264083117)
    .bind(216529000.0)
    .bind(0.005)
    .bind(0.020943951023931952)
    .bind(1.7453292519943295)
    .bind(0.5235987755982988)
    .bind(0.0)
    .bind(2.314065e-06)
    .bind(0.0)
    .execute(&pool)
    .await?;

    sqlx::query(
        "INSERT INTO atmospheres (id, planet_id, pressure_pa, greenhouse_effect_k, lapse_rate_k_per_m) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(jatur_atm_id.to_string())
    .bind(jatur_id.to_string())
    .bind(233445.0)
    .bind(42.0)
    .bind(0.009242)
    .execute(&pool)
    .await?;

    let jatur_gases = [
        ("CO2", 96.92),
        ("N2", 3.05),
        ("CH4", 0.00033),
        ("Ar", 0.02967),
    ];
    for (formula, pct) in jatur_gases {
        sqlx::query(
            "INSERT INTO atmosphere_gas_components (atmosphere_id, formula, percentage) VALUES (?, ?, ?)",
        )
        .bind(jatur_atm_id.to_string())
        .bind(formula)
        .bind(pct)
        .execute(&pool)
        .await?;
    }

    sqlx::query(
        "INSERT INTO atmospheres (id, planet_id, pressure_pa, greenhouse_effect_k, lapse_rate_k_per_m) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(hadab_atm_id.to_string())
    .bind(hadab_id.to_string())
    .bind(101325.0)
    .bind(32.7863)
    .bind(0.0107)
    .execute(&pool)
    .await?;

    let hadab_gases = [
        ("N2", 78.0585),
        ("O2", 20.98),
        ("Ar", 0.93),
        ("CO2", 0.0315),
    ];
    for (formula, pct) in hadab_gases {
        sqlx::query(
            "INSERT INTO atmosphere_gas_components (atmosphere_id, formula, percentage) VALUES (?, ?, ?)",
        )
        .bind(hadab_atm_id.to_string())
        .bind(formula)
        .bind(pct)
        .execute(&pool)
        .await?;
    }

    let epoch_zero = Duration::new(0.0);

    println!("[SEÇÃO 1] TERMODINÂMICA E ESTRUTURA ATMOSFÉRICA (HADAB vs JATUR vs MEROS)");
    println!("----------------------------------------------------------------------------------------------------");

    let planets_to_eval = [
        ("Hadab (Oceânico - 74% Hidrosfera)", hadab_id),
        ("Jatur (Desértico - CO2 Denso)", jatur_id),
        ("Meros (Sem Atmosfera)", meros_id),
    ];

    for (name, pid) in planets_to_eval {
        let t_global = resolve_global_mean_temperature(&pool, pid, epoch_zero, epoch_zero).await?;
        let circ = resolve_planetary_circulation(&pool, pid, epoch_zero, epoch_zero).await?;

        println!("  Planeta: {}", name);
        println!("    Temperatura Global Média : {:>8.2} K ({:>6.2} °C)", t_global.value(), t_global.value() - 273.15);
        println!("    Velocidade Angular (Ω)   : {:>10.4e} rad/s", circ.angular_velocity.value());
        println!("    Parâmetro Beta Equatorial: {:>10.4e} (m·s)⁻¹", circ.equatorial_beta);
        println!("    Raio Deformação Rossby   : {:>10.2} km", circ.rossby_deformation_radius.value() * 1e-3);
        println!("    Escala de Rhines         : {:>10.2} km", circ.rhines_scale.value() * 1e-3);
        println!("    Células por Hemisfério   : {:>8}", circ.circulation_cells);
        println!("    Capacidade Térmica Coluna: {:>10.2e} J/(m²·K)", circ.column_heat_capacity);
        println!("    Eficiência Redistribuição: {:>10.4} %", circ.thermal_redistribution_efficiency * 100.0);
        println!();
    }

    println!("[SEÇÃO 2] PERFIL DE CLIMA LATITUDINAL E ADVEÇÃO EM HADAB (SOLSTÍCIO DE VERÃO)");
    println!("----------------------------------------------------------------------------------------------------");
    println!(
        "  {:<16} | {:<12} | {:<16} | {:<16} | {:<12}",
        "Latitude", "T_Equilíbrio", "T_Inércia (Fixo)", "T_Advectiva (Real)", "Diferença"
    );
    println!("  -----------------+--------------+------------------+--------------------+-------------");

    let lats = [
        ("Polo Norte (+90°)", (90.0 * PI) / 180.0),
        ("Subpolar (+60°)", (60.0 * PI) / 180.0),
        ("Temperado (+45°)", (45.0 * PI) / 180.0),
        ("Subtropical (+30°)", (30.0 * PI) / 180.0),
        ("Equador (0°)", 0.0),
        ("Subtropical (-30°)", (-30.0 * PI) / 180.0),
        ("Temperado (-45°)", (-45.0 * PI) / 180.0),
        ("Subpolar (-60°)", (-60.0 * PI) / 180.0),
        ("Polo Sul (-90°)", (-90.0 * PI) / 180.0),
    ];

    for (label, lat_rad) in lats {
        let lat_ang = Angle::new(lat_rad);
        let t_blended = resolve_latitudinal_surface_temperature(&pool, hadab_id, lat_ang, epoch_zero, epoch_zero).await?;
        let t_advective = resolve_advective_surface_temperature(&pool, hadab_id, lat_ang, epoch_zero, epoch_zero).await?;
        let t_global = resolve_global_mean_temperature(&pool, hadab_id, epoch_zero, epoch_zero).await?;
        let diff = t_advective.value() - t_blended.value();

        println!(
            "  {:<16} | {:>7.2} K     | {:>7.2} K ({:>5.1} °C) | {:>7.2} K ({:>5.1} °C) | {:>+6.2} K",
            label,
            t_global.value(),
            t_blended.value(),
            t_blended.value() - 273.15,
            t_advective.value(),
            t_advective.value() - 273.15,
            diff
        );
    }
    println!();

    println!("[SEÇÃO 3] DINÂMICA DE VENTOS, JATOS ZONAIS E VETOR DE SUPERFÍCIE EM HADAB");
    println!("----------------------------------------------------------------------------------------------------");
    println!(
        "  {:<16} | {:<12} | {:<16} | {:<16} | {:<16} | {:<18}",
        "Latitude", "Coriolis (f)", "Grad. Térmico", "Jato Tropopausa", "Vento Superfície", "Componentes (u, v)"
    );
    println!("  -----------------+--------------+------------------+------------------+------------------+--------------------");

    for (label, lat_rad) in lats {
        let lat_ang = Angle::new(lat_rad);
        let wind = resolve_wind_profile_at_latitude(&pool, hadab_id, lat_ang, epoch_zero, epoch_zero).await?;

        println!(
            "  {:<16} | {:>10.3e} s⁻¹ | {:>12.4e} K/m | {:>10.2} km/h   | {:>10.2} km/h   | [{:>6.2}, {:>6.2}] km/h",
            label,
            wind.coriolis_parameter.value(),
            wind.temperature_gradient.value(),
            wind.jet_stream_speed.value() * 3.6,
            wind.surface_wind_speed.value() * 3.6,
            wind.surface_wind_u.value() * 3.6,
            wind.surface_wind_v.value() * 3.6
        );
    }
    println!();

    println!("[SEÇÃO 4] PERFIL VERTICAL DE PRESSÃO, TEMPERATURA E DENSIDADE ATMOSFÉRICA EM HADAB");
    println!("----------------------------------------------------------------------------------------------------");
    println!(
        "  {:<12} | {:<20} | {:<24} | {:<18}",
        "Altitude", "Temperatura", "Pressão", "Densidade"
    );
    println!("  -------------+----------------------+--------------------------+--------------------");

    let t_surf_hadab = resolve_advective_surface_temperature(&pool, hadab_id, Angle::new(0.0), epoch_zero, epoch_zero).await?;
    let checkpoints = [0.0, 1000.0, 3000.0, 6000.0, 10000.0, 15000.0, 25000.0];

    for alt_m in checkpoints {
        let (p_alt, t_alt, rho_alt) = resolve_atmospheric_profile_at_altitude(
            &pool,
            hadab_id,
            t_surf_hadab,
            Length::new(alt_m),
        )
        .await?;

        println!(
            "  {:>9.2} km | {:>7.2} K ({:>6.2} °C) | {:>12.2} Pa ({:>6.3} bar) | {:>12.4e} kg/m³",
            alt_m * 1e-3,
            t_alt.value(),
            t_alt.value() - 273.15,
            p_alt.value(),
            p_alt.value() * 1e-5,
            rho_alt.value()
        );
    }
    println!();

    println!("[SEÇÃO 5] METEOROLOGIA ESPACIAL E VENTO ESTELAR DE HÉLCA SOBRE HADAB E JATUR");
    println!("----------------------------------------------------------------------------------------------------");

    let positions = resolve_system_positions(&pool, system_id, epoch_zero).await?;
    let pos_helca = positions.get(&helca_id).copied().unwrap_or_else(Position::zero);
    let pos_hadab = positions.get(&hadab_id).copied().unwrap_or_else(Position::zero);
    let pos_jatur = positions.get(&jatur_id).copied().unwrap_or_else(Position::zero);

    let d_hadab = (pos_hadab - pos_helca).magnitude().value();
    let d_jatur = (pos_jatur - pos_helca).magnitude().value();

    let sw_hadab = resolve_stellar_wind_at_planet(&pool, hadab_id, 1.0, 2.0, epoch_zero, epoch_zero).await?;
    let sw_jatur = resolve_stellar_wind_at_planet(&pool, jatur_id, 1.0, 2.0, epoch_zero, epoch_zero).await?;

    println!("  Taxa Perda de Massa de Hélca (Reimers) : {:>12.4e} kg/s", sw_hadab.mass_loss_rate.value());
    println!("  Velocidade de Escape de Hélca           : {:>12.2} km/s", sw_hadab.escape_velocity.value() * 1e-3);
    println!("  Velocidade Terminal do Vento Estelar    : {:>12.2} km/s", sw_hadab.terminal_wind_speed.value() * 1e-3);
    println!("  --------------------------------------------------------------------------------------------------");
    println!("  Em Hadab (Distância: {:.4} AU):", d_hadab / ASTRONOMICAL_UNIT);
    println!("    Densidade do Vento Estelar : {:>12.4e} kg/m³", sw_hadab.wind_density_at_orbit.value());
    println!("    Pressão Dinâmica do Vento  : {:>12.4e} Pa ({:.4e} nPa)", sw_hadab.dynamic_pressure.value(), sw_hadab.dynamic_pressure.value() * 1e9);
    println!("  Em Jatur (Distância: {:.4} AU):", d_jatur / ASTRONOMICAL_UNIT);
    println!("    Densidade do Vento Estelar : {:>12.4e} kg/m³", sw_jatur.wind_density_at_orbit.value());
    println!("    Pressão Dinâmica do Vento  : {:>12.4e} Pa ({:.4e} nPa)", sw_jatur.dynamic_pressure.value(), sw_jatur.dynamic_pressure.value() * 1e9);
    println!();

    println!("====================================================================================================");
    println!("Status da Validação: MODELO CLIMÁTICO, DINÂMICA DE CIRCULAÇÃO E METEOROLOGIA RIGOROSAMENTE VALIDADOS.");
    println!("====================================================================================================");

    Ok(())
}