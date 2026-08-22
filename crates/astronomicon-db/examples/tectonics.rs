use astronomicon_app::{
    resolve_global_mean_temperature, resolve_planetary_core, resolve_planetary_geology,
    resolve_seismic_diagnostics, resolve_tidal_diagnostics,
};
use astronomicon_core::domain::Planet;
use astronomicon_core::units::constants::SECONDS_PER_YEAR;
use astronomicon_core::units::Duration;
use astronomicon_db::repositories::{planet_repository, universe_state_repository};
use astronomicon_db::save::initialize_save;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://database/astronomicon.db".to_string());
    let pool = initialize_save(&db_url).await?;

    let universe_state_row = universe_state_repository::get(&pool)
        .await?
        .expect("universe_state not found");
    let universe_epoch = Duration::new(universe_state_row.seconds_since_j2000_epoch);
    let at_epoch = Duration::new(0.0);

    let planet_rows = planet_repository::list_all(&pool).await?;
    let mut planets: Vec<Planet> = planet_rows
        .into_iter()
        .map(Planet::try_from)
        .collect::<Result<Vec<_>, _>>()?;

    let target_order = ["Hadab", "Jatur", "Meros", "Avizina", "Jena"];
    planets.sort_by_key(|p| {
        target_order
            .iter()
            .position(|&name| name == p.name())
            .unwrap_or(usize::MAX)
    });

    println!("{}", "=".repeat(100));
    println!(
        "{:^100}",
        "ASTRONOMICON - RELATÓRIO GEOFÍSICO DE TECTONISMO, REOLOGIA E SISMICIDADE"
    );
    println!(
        "{:^100}",
        "(Hadab, Jatur, Meros, Avizina e Jena)"
    );
    println!("{}", "=".repeat(100));

    for planet in &planets {
        if !target_order.contains(&planet.name()) {
            continue;
        }

        let planet_id = planet.id();

        let surface_temp =
            resolve_global_mean_temperature(&pool, planet_id, universe_epoch, at_epoch).await?;
        let core_diag = resolve_planetary_core(&pool, planet_id, universe_epoch, at_epoch).await?;
        let geology_diag =
            resolve_planetary_geology(&pool, planet_id, universe_epoch, at_epoch).await?;
        let seismic_diag =
            resolve_seismic_diagnostics(&pool, planet_id, universe_epoch, at_epoch).await?;
        let tidal_diag =
            resolve_tidal_diagnostics(&pool, planet_id, universe_epoch, at_epoch).await?;

        let temp_k = surface_temp.value();
        let temp_c = temp_k - 273.15;

        let q_conv = core_diag.convective_heat_flux.value();
        let q_conv_mw = q_conv * 1000.0;

        let q_rad = core_diag.radiogenic_heat_flux.value();
        let q_rad_mw = q_rad * 1000.0;

        let q_tide = core_diag.tidal_heat_flux.value();
        let q_tide_mw = q_tide * 1000.0;

        let q_tot = core_diag.total_surface_heat_flux.value();
        let q_tot_mw = q_tot * 1000.0;

        let z_lith_km = geology_diag.lithosphere_thickness.value() / 1000.0;

        let plate_count = geology_diag.plate_count;
        let plate_v = geology_diag.plate_velocity.value();
        let plate_v_cm_yr = plate_v * SECONDS_PER_YEAR * 100.0;

        let p_tect = seismic_diag.tectonic_seismic_energy.value();
        let p_tect_mw = p_tect / 1.0e6;

        let p_seis_tide = seismic_diag.tidal_seismic_energy.value();
        let p_seis_tide_mw = p_seis_tide / 1.0e6;

        let p_seis_tot = seismic_diag.total_seismic_energy.value();
        let p_seis_tot_mw = p_seis_tot / 1.0e6;

        let annual_seismic = p_seis_tot * SECONDS_PER_YEAR;

        let tidal_stress = seismic_diag.tidal_stress_amplitude.value();
        let tidal_stress_kpa = tidal_stress / 1000.0;

        let bulge_h = seismic_diag.tidal_bulge_height.value();

        let tidal_pow = tidal_diag.tidal_heating_energy.value();
        let tidal_pow_mw = tidal_pow / 1.0e6;

        println!("{}", "-".repeat(100));
        println!(
            "CORPO CELESTE: {:<12} | REGIME TECTÔNICO: {:?}",
            planet.name(),
            geology_diag.tectonic_regime
        );
        println!("{}", "-".repeat(100));
        println!("[1] TERMODINÂMICA DA LITOSFERA E FLUXO TÉRMICO:");
        println!(
            "  Temperatura de Superfície      : {:>10.2} K ({:>6.2} °C)",
            temp_k, temp_c
        );
        println!(
            "  Fluxo Térmico Convectivo Core  :  {:>10.4e} W/m² ({:>6.2} mW/m²)",
            q_conv, q_conv_mw
        );
        println!(
            "  Fluxo Radiogênico Mantélico    :  {:>10.4e} W/m² ({:>6.2} mW/m²)",
            q_rad, q_rad_mw
        );
        println!(
            "  Fluxo Térmico de Maré          :  {:>10.4e} W/m² ({:>6.2} mW/m²)",
            q_tide, q_tide_mw
        );
        println!(
            "  Fluxo Geotérmico Total Superf. :  {:>10.4e} W/m² ({:>6.2} mW/m²)",
            q_tot, q_tot_mw
        );
        println!(
            "  Espessura Mecânica da Litosfera: {:>10.2} km",
            z_lith_km
        );
        println!();
        println!("[2] CINEMÁTICA E DINÂMICA DE PLACAS:");
        println!(
            "  Número de Placas Maiores       :         {:>2}",
            plate_count
        );
        println!(
            "  Velocidade RMS das Placas      :  {:>10.4e} m/s ({:>6.2} cm/ano)",
            plate_v, plate_v_cm_yr
        );
        println!();
        println!("[3] BALANÇO DE ENERGIA SÍSMICA E TENSÕES DE MARÉ:");
        println!(
            "  Potência Sísmica Tectônica     :  {:>10.4e} W ({:>8.2} MW)",
            p_tect, p_tect_mw
        );
        println!(
            "  Potência Sísmica de Maré       :  {:>10.4e} W ({:>8.2} MW)",
            p_seis_tide, p_seis_tide_mw
        );
        println!(
            "  Potência Sísmica Total Global  :  {:>10.4e} W ({:>8.2} MW)",
            p_seis_tot, p_seis_tot_mw
        );
        println!(
            "  Taxa Anual de Liberação Sísmica:  {:>10.4e} J/ano",
            annual_seismic
        );
        println!(
            "  Amplitude de Tensão de Maré    :  {:>10.4e} Pa ({:>8.2} kPa)",
            tidal_stress, tidal_stress_kpa
        );
        println!(
            "  Altura do Bojo de Maré         : {:>10.4} m",
            bulge_h
        );
        println!(
            "  Potência Dissipada por Maré    :  {:>10.4e} W ({:>8.2} MW)",
            tidal_pow, tidal_pow_mw
        );
        println!();
    }

    println!("{}", "=".repeat(100));
    println!("Status da Validação: TECTONISMO, REOLOGIA E SISMICIDADE CONCLUÍDOS COM SUCESSO.");
    println!("{}", "=".repeat(100));

    Ok(())
}