use astronomicon_app::climate::resolve_global_mean_temperature;
use astronomicon_app::geology::resolve_planetary_geology;
use astronomicon_app::geophysics::resolve_planetary_core;
use astronomicon_app::seismology::resolve_seismic_diagnostics;
use astronomicon_app::tidal::resolve_tidal_diagnostics;
use astronomicon_core::units::Duration;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    let hadab_hydro_id = Uuid::parse_str("12345678-1234-1234-1234-1234567890ab")?;

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
         argument_periapsis_rad, mean_anomaly_at_epoch_rad, oblateness_j2, core_mass_fraction, \
         radioactive_heating_rate, mantle_hydration_fraction, love_number_k2, tidal_dissipation_factor_q) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
    .bind(0.62)
    .bind(1.45)
    .bind(0.0002)
    .bind(0.32)
    .bind(50.0)
    .execute(&pool)
    .await?;

    sqlx::query(
        "INSERT INTO planets (id, star_system_id, parent_star_id, name, kind, mass_kg, equatorial_radius_m, \
         polar_radius_m, rotation_period_s, axial_tilt_rad, geometric_albedo, bond_albedo, thermal_inertia, \
         solstice_true_anomaly_rad, semi_major_axis_m, eccentricity, inclination_rad, longitude_ascending_node_rad, \
         argument_periapsis_rad, mean_anomaly_at_epoch_rad, oblateness_j2, core_mass_fraction, \
         radioactive_heating_rate, mantle_hydration_fraction, love_number_k2, tidal_dissipation_factor_q) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
    .bind(0.30)
    .bind(0.95)
    .bind(0.000040)
    .bind(0.32)
    .bind(100.0)
    .execute(&pool)
    .await?;

    sqlx::query(
        "INSERT INTO planets (id, star_system_id, parent_star_id, name, kind, mass_kg, equatorial_radius_m, \
         polar_radius_m, rotation_period_s, axial_tilt_rad, geometric_albedo, bond_albedo, thermal_inertia, \
         solstice_true_anomaly_rad, semi_major_axis_m, eccentricity, inclination_rad, longitude_ascending_node_rad, \
         argument_periapsis_rad, mean_anomaly_at_epoch_rad, oblateness_j2, core_mass_fraction, \
         radioactive_heating_rate, mantle_hydration_fraction, love_number_k2, tidal_dissipation_factor_q) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
    .bind(0.345)
    .bind(1.10)
    .bind(0.000150)
    .bind(0.32)
    .bind(100.0)
    .execute(&pool)
    .await?;

    sqlx::query(
        "INSERT INTO planets (id, star_system_id, parent_planet_id, name, kind, mass_kg, equatorial_radius_m, \
         polar_radius_m, rotation_period_s, axial_tilt_rad, geometric_albedo, bond_albedo, thermal_inertia, \
         solstice_true_anomaly_rad, semi_major_axis_m, eccentricity, inclination_rad, longitude_ascending_node_rad, \
         argument_periapsis_rad, mean_anomaly_at_epoch_rad, oblateness_j2, core_mass_fraction, \
         radioactive_heating_rate, mantle_hydration_fraction, love_number_k2, tidal_dissipation_factor_q) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
    .bind(4.170363e-6)
    .bind(0.08)
    .bind(1.00)
    .bind(0.0)
    .bind(0.25)
    .bind(50.0)
    .execute(&pool)
    .await?;

    sqlx::query(
        "INSERT INTO planets (id, star_system_id, parent_planet_id, name, kind, mass_kg, equatorial_radius_m, \
         polar_radius_m, rotation_period_s, axial_tilt_rad, geometric_albedo, bond_albedo, thermal_inertia, \
         solstice_true_anomaly_rad, semi_major_axis_m, eccentricity, inclination_rad, longitude_ascending_node_rad, \
         argument_periapsis_rad, mean_anomaly_at_epoch_rad, oblateness_j2, core_mass_fraction, \
         radioactive_heating_rate, mantle_hydration_fraction, love_number_k2, tidal_dissipation_factor_q) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
    .bind(2.314065e-6)
    .bind(0.12)
    .bind(1.00)
    .bind(0.0)
    .bind(0.25)
    .bind(50.0)
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
    .bind(37.5151)
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

    sqlx::query(
        "INSERT INTO hydrospheres (id, planet_id, average_depth_m, surface_coverage_fraction, salinity_or_solute_mass_fraction) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(hadab_hydro_id.to_string())
    .bind(hadab_id.to_string())
    .bind(3800.0)
    .bind(0.74)
    .bind(0.035)
    .execute(&pool)
    .await?;

    sqlx::query(
        "INSERT INTO hydrosphere_components (hydrosphere_id, formula, percentage) VALUES (?, ?, ?)",
    )
    .bind(hadab_hydro_id.to_string())
    .bind("H2O")
    .bind(100.0)
    .execute(&pool)
    .await?;

    let sim_age = Duration::new(4.5e9 * 365.25 * 86400.0);
    let epoch_zero = Duration::new(0.0);

    let targets = [
        ("Hadab", hadab_id),
        ("Jatur", jatur_id),
        ("Meros", meros_id),
        ("Avizina", avizina_id),
        ("Jena", jena_id),
    ];

    println!("====================================================================================================");
    println!("             ASTRONOMICON - RELATÓRIO GEOFÍSICO DE TECTONISMO, REOLOGIA E SISMICIDADE               ");
    println!("                              (Hadab, Jatur, Meros, Avizina e Jena)                                 ");
    println!("====================================================================================================");

    for (name, pid) in targets {
        let t_surf = resolve_global_mean_temperature(&pool, pid, sim_age, epoch_zero).await?;
        let core = resolve_planetary_core(&pool, pid, sim_age, epoch_zero).await?;
        let tidal = resolve_tidal_diagnostics(&pool, pid, sim_age, epoch_zero).await?;
        let geo = resolve_planetary_geology(&pool, pid, sim_age, epoch_zero).await?;
        let seis = resolve_seismic_diagnostics(&pool, pid, sim_age, epoch_zero).await?;

        println!("----------------------------------------------------------------------------------------------------");
        println!("CORPO CELESTE: {:<12} | REGIME TECTÔNICO: {:?}", name, geo.tectonic_regime);
        println!("----------------------------------------------------------------------------------------------------");
        println!("[1] TERMODINÂMICA DA LITOSFERA E FLUXO TÉRMICO:");
        println!("  Temperatura de Superfície      : {:>10.2} K ({:>6.2} °C)", t_surf.value(), t_surf.value() - 273.15);
        println!("  Fluxo Térmico Convectivo Core  : {:>10.4e} W/m² ({:>6.2} mW/m²)", core.convective_heat_flux.value(), core.convective_heat_flux.value() * 1e3);
        println!("  Fluxo Radiogênico Mantélico    : {:>10.4e} W/m² ({:>6.2} mW/m²)", core.radiogenic_heat_flux.value(), core.radiogenic_heat_flux.value() * 1e3);
        println!("  Fluxo Térmico de Maré          : {:>10.4e} W/m² ({:>6.2} mW/m²)", core.tidal_heat_flux.value(), core.tidal_heat_flux.value() * 1e3);
        println!("  Fluxo Geotérmico Total Superf. : {:>10.4e} W/m² ({:>6.2} mW/m²)", core.total_surface_heat_flux.value(), core.total_surface_heat_flux.value() * 1e3);
        println!("  Espessura Mecânica da Litosfera: {:>10.2} km", geo.lithosphere_thickness.value() * 1e-3);
        println!();
        println!("[2] CINEMÁTICA E DINÂMICA DE PLACAS:");
        println!("  Número de Placas Maiores       : {:>10}", geo.plate_count);
        println!("  Velocidade RMS das Placas      : {:>10.4e} m/s ({:>6.2} cm/ano)", geo.plate_velocity.value(), geo.plate_velocity.value() * 31557600.0 * 100.0);
        println!();
        println!("[3] BALANÇO DE ENERGIA SÍSMICA E TENSÕES DE MARÉ:");
        println!("  Potência Sísmica Tectônica     : {:>10.4e} W ({:>8.2} MW)", geo.native_seismic_energy.value(), geo.native_seismic_energy.value() * 1e-6);
        println!("  Potência Sísmica de Maré       : {:>10.4e} W ({:>8.2} MW)", geo.tidal_seismic_energy.value(), geo.tidal_seismic_energy.value() * 1e-6);
        println!("  Potência Sísmica Total Global  : {:>10.4e} W ({:>8.2} MW)", seis.total_seismic_energy.value(), seis.total_seismic_energy.value() * 1e-6);
        println!("  Taxa Anual de Liberação Sísmica: {:>10.4e} J/ano", seis.total_seismic_energy.value() * 31557600.0);
        println!("  Amplitude de Tensão de Maré    : {:>10.4e} Pa ({:>8.2} kPa)", seis.tidal_stress_amplitude.value(), seis.tidal_stress_amplitude.value() * 1e-3);
        println!("  Altura do Bojo de Maré         : {:>10.4} m", seis.tidal_bulge_height.value());
        println!("  Potência Dissipada por Maré    : {:>10.4e} W ({:>8.2} MW)", tidal.tidal_heating_energy.value(), tidal.tidal_heating_energy.value() * 1e-6);
        println!();
    }

    println!("====================================================================================================");
    println!("Status da Validação: TECTONISMO, REOLOGIA E SISMICIDADE CONCLUÍDOS COM SUCESSO.");
    println!("====================================================================================================");

    Ok(())
}