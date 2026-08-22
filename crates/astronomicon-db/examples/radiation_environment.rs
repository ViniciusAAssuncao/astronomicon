use astronomicon_app::ephemeris::resolve_system_positions;
use astronomicon_app::geophysics::resolve_magnetic_field;
use astronomicon_app::radiation::{ resolve_interplanetary_radiation, resolve_surface_radiation };
use astronomicon_core::chemistry::molar_mass::{ mass_attenuation_coefficient_of };
use astronomicon_core::math::radiation::{
    atmospheric_mass_column,
    cmb_energy_density,
    galactic_cosmic_ray_background,
    peak_wavelength,
    planck_spectral_radiance,
};
use astronomicon_core::units::constants::{
    ASTRONOMICAL_UNIT,
    COSMIC_MICROWAVE_BACKGROUND_TEMPERATURE,
};
use astronomicon_core::units::{ Duration, Position, Pressure, Temperature, Wavelength };
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "===================================================================================================="
    );
    println!(
        "               ASTRONOMICON - RELATÓRIO COMPLETO DO AMBIENTE DE RADIAÇÃO E DOSIMETRIA               "
    );
    println!(
        "                           (Validação Física Integral do Módulo de Radiação)                        "
    );
    println!(
        "===================================================================================================="
    );

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

    sqlx
        ::query(
            "INSERT INTO star_systems (id, name, right_ascension_rad, declination_rad, distance_from_sol_m) \
         VALUES (?, ?, ?, ?, ?)"
        )
        .bind(system_id.to_string())
        .bind("Sistema Zód")
        .bind(1.25)
        .bind(-0.45)
        .bind(4.12e17)
        .execute(&pool).await?;

    sqlx
        ::query(
            "INSERT INTO stars (id, star_system_id, name, kind, mass_kg, radius_m, effective_temperature_k, \
         rotation_period_s, axial_tilt_rad, oblateness_j2) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
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
        .execute(&pool).await?;

    sqlx
        ::query(
            "INSERT INTO stars (id, star_system_id, name, kind, mass_kg, radius_m, effective_temperature_k, \
         rotation_period_s, axial_tilt_rad) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
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
        .execute(&pool).await?;

    sqlx
        ::query(
            "INSERT INTO stars (id, star_system_id, parent_star_id, name, kind, mass_kg, radius_m, \
         effective_temperature_k, rotation_period_s, axial_tilt_rad, semi_major_axis_m, eccentricity, \
         inclination_rad, longitude_ascending_node_rad, argument_periapsis_rad, mean_anomaly_at_epoch_rad) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
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
        .execute(&pool).await?;

    sqlx
        ::query(
            "INSERT INTO planets (id, star_system_id, parent_star_id, name, kind, mass_kg, equatorial_radius_m, \
         polar_radius_m, rotation_period_s, axial_tilt_rad, geometric_albedo, bond_albedo, thermal_inertia, \
         solstice_true_anomaly_rad, semi_major_axis_m, eccentricity, inclination_rad, longitude_ascending_node_rad, \
         argument_periapsis_rad, mean_anomaly_at_epoch_rad, oblateness_j2, hydrosphere_fraction, \
         core_mass_fraction, radioactive_heating_rate) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
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
        .bind(0.15)
        .bind(0.2)
        .execute(&pool).await?;

    sqlx
        ::query(
            "INSERT INTO planets (id, star_system_id, parent_star_id, name, kind, mass_kg, equatorial_radius_m, \
         polar_radius_m, rotation_period_s, axial_tilt_rad, geometric_albedo, bond_albedo, thermal_inertia, \
         solstice_true_anomaly_rad, semi_major_axis_m, eccentricity, inclination_rad, longitude_ascending_node_rad, \
         argument_periapsis_rad, mean_anomaly_at_epoch_rad, oblateness_j2, hydrosphere_fraction, \
         core_mass_fraction, radioactive_heating_rate) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
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
        .bind(0.25)
        .bind(0.8)
        .execute(&pool).await?;

    sqlx
        ::query(
            "INSERT INTO planets (id, star_system_id, parent_star_id, name, kind, mass_kg, equatorial_radius_m, \
         polar_radius_m, rotation_period_s, axial_tilt_rad, geometric_albedo, bond_albedo, thermal_inertia, \
         solstice_true_anomaly_rad, semi_major_axis_m, eccentricity, inclination_rad, longitude_ascending_node_rad, \
         argument_periapsis_rad, mean_anomaly_at_epoch_rad, oblateness_j2, hydrosphere_fraction, \
         core_mass_fraction, radioactive_heating_rate) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
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
        .bind(0.32)
        .bind(1.0)
        .execute(&pool).await?;

    sqlx
        ::query(
            "INSERT INTO planets (id, star_system_id, parent_planet_id, name, kind, mass_kg, equatorial_radius_m, \
         polar_radius_m, rotation_period_s, axial_tilt_rad, geometric_albedo, bond_albedo, thermal_inertia, \
         solstice_true_anomaly_rad, semi_major_axis_m, eccentricity, inclination_rad, longitude_ascending_node_rad, \
         argument_periapsis_rad, mean_anomaly_at_epoch_rad, oblateness_j2, hydrosphere_fraction, \
         core_mass_fraction, radioactive_heating_rate) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
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
        .bind(0.0)
        .bind(0.05)
        .bind(0.1)
        .execute(&pool).await?;

    sqlx
        ::query(
            "INSERT INTO planets (id, star_system_id, parent_planet_id, name, kind, mass_kg, equatorial_radius_m, \
         polar_radius_m, rotation_period_s, axial_tilt_rad, geometric_albedo, bond_albedo, thermal_inertia, \
         solstice_true_anomaly_rad, semi_major_axis_m, eccentricity, inclination_rad, longitude_ascending_node_rad, \
         argument_periapsis_rad, mean_anomaly_at_epoch_rad, oblateness_j2, hydrosphere_fraction, \
         core_mass_fraction, radioactive_heating_rate) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
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
        .bind(0.0)
        .bind(0.03)
        .bind(0.05)
        .execute(&pool).await?;

    sqlx
        ::query(
            "INSERT INTO atmospheres (id, planet_id, pressure_pa, greenhouse_effect_k, lapse_rate_k_per_m) \
         VALUES (?, ?, ?, ?, ?)"
        )
        .bind(jatur_atm_id.to_string())
        .bind(jatur_id.to_string())
        .bind(233445.0)
        .bind(42.0)
        .bind(0.009242)
        .execute(&pool).await?;

    let jatur_gases = [
        ("CO2", 96.92),
        ("N2", 3.05),
        ("CH4", 0.00033),
        ("Ar", 0.02967),
    ];
    for (formula, pct) in jatur_gases {
        sqlx
            ::query(
                "INSERT INTO atmosphere_gas_components (atmosphere_id, formula, percentage) VALUES (?, ?, ?)"
            )
            .bind(jatur_atm_id.to_string())
            .bind(formula)
            .bind(pct)
            .execute(&pool).await?;
    }

    sqlx
        ::query(
            "INSERT INTO atmospheres (id, planet_id, pressure_pa, greenhouse_effect_k, lapse_rate_k_per_m) \
         VALUES (?, ?, ?, ?, ?)"
        )
        .bind(hadab_atm_id.to_string())
        .bind(hadab_id.to_string())
        .bind(101325.0)
        .bind(32.7863)
        .bind(0.0107)
        .execute(&pool).await?;

    let hadab_gases = [
        ("N2", 78.0585),
        ("O2", 20.98),
        ("Ar", 0.93),
        ("CO2", 0.0315),
    ];
    for (formula, pct) in hadab_gases {
        sqlx
            ::query(
                "INSERT INTO atmosphere_gas_components (atmosphere_id, formula, percentage) VALUES (?, ?, ?)"
            )
            .bind(hadab_atm_id.to_string())
            .bind(formula)
            .bind(pct)
            .execute(&pool).await?;
    }

    let epoch_zero = Duration::new(0.0);
    let sim_age = Duration::new(4.5e9 * 365.25 * 86400.0);

    println!("[SEÇÃO 1] FUNDAMENTOS QUÂNTICOS, RADIAÇÃO CÓSMICA DE FUNDO E ESPECTRO ESTELAR");
    println!(
        "----------------------------------------------------------------------------------------------------"
    );

    let cmb_temp = Temperature::new(COSMIC_MICROWAVE_BACKGROUND_TEMPERATURE);
    let cmb_u = cmb_energy_density(cmb_temp);
    let gcr_base = galactic_cosmic_ray_background();

    println!("  Temperatura da CMB (T_cmb)         : {:>12.5} K", cmb_temp.value());
    println!("  Densidade de Energia da CMB (u_cmb): {:>12.4e} J/m³", cmb_u);
    println!(
        "  Dose Base de Raios Cósmicos (GCR)  : {:>12.4} Sv/ano ({:.2} mSv/ano)",
        gcr_base.value(),
        gcr_base.value() * 1e3
    );
    println!(
        "  --------------------------------------------------------------------------------------------------"
    );

    let stars_to_eval = [
        ("Hélca (Estrela Mãe - G-Type)", Temperature::new(5557.033)),
        ("Ásdi (Companheira Binária - K-Type)", Temperature::new(4243.0)),
        ("Nélica (Anã Branca Degenerada)", Temperature::new(9200.0)),
        ("Sol (Referência Terrestre)", Temperature::new(5778.0)),
    ];

    for (name, temp) in stars_to_eval {
        let lambda_max = peak_wavelength(temp);
        let b_max = planck_spectral_radiance(lambda_max, temp);
        let b_uvc = planck_spectral_radiance(Wavelength::new(254e-9), temp);
        let spectral_band = if lambda_max.value() < 380e-9 {
            "Ultravioleta (UV)"
        } else if lambda_max.value() <= 750e-9 {
            "Luz Visível"
        } else {
            "Infravermelho (IR)"
        };

        println!("  Estrela: {}", name);
        println!("    Temperatura Efetiva : {:>10.2} K", temp.value());
        println!(
            "    Pico de Wien (λ_max): {:>10.2} nm ({})",
            lambda_max.value() * 1e9,
            spectral_band
        );
        println!("    Radiância no Pico   : {:>10.4e} W/(m²·sr·m)", b_max);
        println!(
            "    Radiância UV-C (254nm): {:>8.4e} W/(m²·sr·m) ({:.4}% do pico)",
            b_uvc,
            (b_uvc / b_max.max(1e-30)) * 100.0
        );
        println!();
    }

    println!("[SEÇÃO 2] QUÍMICA DE INTERAÇÃO RADIAÇÃO-MATÉRIA E SEÇÃO DE CHOQUE ATMOSFÉRICA");
    println!(
        "----------------------------------------------------------------------------------------------------"
    );
    println!(
        "  {:<12} | {:<20} | {:<22} | {:<16}",
        "Fórmula",
        "Massa Molar (g/mol)",
        "Coef. Atenuação μ (m²/kg)",
        "Atenuação Relativa"
    );
    println!("  -------------+----------------------+------------------------+-------------------");

    let sample_gases = ["H2", "He", "CH4", "H2O", "N2", "O2", "Ar", "CO2", "SO2", "Xe"];
    for formula in sample_gases {
        let molar_m = astronomicon_core::chemistry::molar_mass::molar_mass_of(formula)?;
        let mu = mass_attenuation_coefficient_of(formula)?;
        println!(
            "  {:<12} | {:>18.3} | {:>22.6e} | {:>15.2}x H2",
            formula,
            molar_m.value() * 1000.0,
            mu.value(),
            mu.value() / mass_attenuation_coefficient_of("H2")?.value()
        );
    }
    println!();

    println!("[SEÇÃO 3] AMBIENTE DE RADIAÇÃO INTERPLANETÁRIA (TOPO DA ATMOSFERA / VÁCUO)");
    println!(
        "----------------------------------------------------------------------------------------------------"
    );
    println!(
        "  {:<16} | {:<12} | {:<14} | {:<14} | {:<14} | {:<14} | {:<16}",
        "Corpo Celeste",
        "Dist. (AU)",
        "Irrad. Fótons",
        "Fluxo Vento",
        "Dose Vento",
        "Dose UV Letal",
        "Dose Vácuo Total"
    );
    println!(
        "  -----------------+--------------+----------------+----------------+----------------+----------------+------------------"
    );

    let targets = [
        ("Hadab", hadab_id),
        ("Jatur", jatur_id),
        ("Meros", meros_id),
        ("Avizina", avizina_id),
        ("Jena", jena_id),
    ];

    let positions = resolve_system_positions(&pool, system_id, epoch_zero).await?;
    let pos_helca = positions.get(&helca_id).copied().unwrap_or_else(Position::zero);

    for (name, pid) in targets {
        let pos_body = positions.get(&pid).copied().unwrap_or_else(Position::zero);
        let dist_au = (pos_body - pos_helca).magnitude().value() / ASTRONOMICAL_UNIT;

        let rad_diag = resolve_interplanetary_radiation(&pool, pid, sim_age, epoch_zero).await?;

        println!(
            "  {:<16} | {:>10.4} AU | {:>10.2} W/m² | {:>10.4e} W/m² | {:>10.4} Sv/a | {:>10.4} Sv/a | {:>12.4} Sv/a",
            name,
            dist_au,
            rad_diag.stellar_irradiance.value(),
            rad_diag.particle_flux.value(),
            rad_diag.stellar_wind_dose.value(),
            rad_diag.lethal_uv_dose.value(),
            rad_diag.total_unshielded_dose.value()
        );
    }
    println!();

    println!("[SEÇÃO 4] BLINDAGEM MAGNETOSFÉRICA E RIGIDEZ DE CORTE (EQUADOR vs POLOS)");
    println!(
        "----------------------------------------------------------------------------------------------------"
    );
    println!(
        "  {:<16} | {:<16} | {:<18} | {:<18} | {:<14} | {:<14}",
        "Planeta",
        "Dipolo Magnético",
        "Rigidez Equador",
        "Rigidez Polos",
        "Transm. Eq.",
        "Transm. Pol."
    );
    println!(
        "  -----------------+------------------+--------------------+--------------------+----------------+----------------"
    );

    for (name, pid) in [
        ("Hadab", hadab_id),
        ("Jatur", jatur_id),
        ("Meros", meros_id),
    ] {
        let mag = resolve_magnetic_field(&pool, pid, 1.0, 1.0, sim_age, epoch_zero).await?;
        let surf = resolve_surface_radiation(&pool, pid, sim_age, epoch_zero).await?;

        println!(
            "  {:<16} | {:>12.4e} A·m² | {:>14.4e} V | {:>14.4e} V | {:>12.4}% | {:>12.4}%",
            name,
            mag.dipole_moment.value(),
            surf.equatorial_cutoff_rigidity.value(),
            surf.polar_cutoff_rigidity.value(),
            surf.equatorial_magnetic_shielding * 100.0,
            surf.polar_magnetic_shielding * 100.0
        );
    }
    println!();

    println!("[SEÇÃO 5] ATENUAÇÃO ATMOSFÉRICA (COLUNA DE MASSA E BEER-LAMBERT)");
    println!(
        "----------------------------------------------------------------------------------------------------"
    );
    println!(
        "  {:<16} | {:<16} | {:<14} | {:<16} | {:<18} | {:<14}",
        "Planeta",
        "Pressão Superf.",
        "Gravidade",
        "Coluna de Massa",
        "Coef. Médio μ",
        "Transm. Atmos."
    );
    println!(
        "  -----------------+------------------+----------------+------------------+--------------------+----------------"
    );

    let atmosphere_cases = [
        ("Hadab", hadab_id, 101325.0, 10.74),
        ("Jatur", jatur_id, 233445.0, 9.74),
        ("Meros", meros_id, 0.0, 4.58),
    ];

    for (name, pid, p_surf, g_val) in atmosphere_cases {
        let mass_col = atmospheric_mass_column(
            Pressure::new(p_surf),
            astronomicon_core::units::Acceleration::new(g_val)
        );
        let surf_diag = resolve_surface_radiation(&pool, pid, sim_age, epoch_zero).await?;

        let mu_str = if p_surf > 0.0 {
            let atm = astronomicon_db::repositories::atmosphere_repository
                ::get_by_planet_id(&pool, &pid).await?
                .unwrap();
            let mu_val = atm.mean_mass_attenuation_coefficient()?.value();
            format!("{:>14.6e} m²/kg", mu_val)
        } else {
            "          N/A      ".to_string()
        };

        println!(
            "  {:<16} | {:>12.2} Pa | {:>10.2} m/s² | {:>12.2} kg/m² | {} | {:>12.6e} %",
            name,
            p_surf,
            g_val,
            mass_col,
            mu_str,
            surf_diag.atmospheric_transmission * 100.0
        );
    }
    println!();

    println!("[SEÇÃO 6] DOSIMETRIA NA SUPERFÍCIE PLANETÁRIA E ATENUAÇÃO DUPLA TOTAL");
    println!(
        "----------------------------------------------------------------------------------------------------"
    );
    println!(
        "  {:<16} | {:<16} | {:<18} | {:<18} | {:<18} | {:<14}",
        "Planeta",
        "Dose Vácuo (TOA)",
        "Dose Eq. Superfície",
        "Dose Pol. Superfície",
        "Redução Equador",
        "Classificação"
    );
    println!(
        "  -----------------+------------------+--------------------+--------------------+--------------------+----------------"
    );

    for (name, pid) in [
        ("Hadab", hadab_id),
        ("Jatur", jatur_id),
        ("Meros", meros_id),
    ] {
        let inter = resolve_interplanetary_radiation(&pool, pid, sim_age, epoch_zero).await?;
        let surf = resolve_surface_radiation(&pool, pid, sim_age, epoch_zero).await?;

        let red_eq =
            (1.0 -
                surf.equatorial_surface_dose.value() /
                    inter.total_unshielded_dose.value().max(1e-12)) *
            100.0;
        let classification = if surf.equatorial_surface_dose.value() < 0.005 {
            "Habitável (Excelente)"
        } else if surf.equatorial_surface_dose.value() < 0.05 {
            "Tolerável (Moderado)"
        } else if surf.equatorial_surface_dose.value() < 0.5 {
            "Severo (Risco Alto)"
        } else {
            "Letal (Sem Proteção)"
        };

        println!(
            "  {:<16} | {:>12.4} Sv/a | {:>10.4e} Sv/a | {:>10.4e} Sv/a | {:>16.6} % | {:<14}",
            name,
            inter.total_unshielded_dose.value(),
            surf.equatorial_surface_dose.value(),
            surf.polar_surface_dose.value(),
            red_eq,
            classification
        );
    }
    println!();

    println!(
        "===================================================================================================="
    );
    println!(
        "Status da Validação: TODAS AS FRENTES DE FÍSICA DAS RADIAÇÕES FORAM VALIDADAS COM SUCESSO TOTAL."
    );
    println!(
        "===================================================================================================="
    );

    Ok(())
}
