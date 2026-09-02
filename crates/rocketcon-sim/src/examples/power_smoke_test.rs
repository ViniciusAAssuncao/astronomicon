use astronomicon_app::climate::resolve_irradiance_at_position;
use astronomicon_app::AppContext;
use astronomicon_core::math::eclipse::is_in_cylindrical_shadow;
use astronomicon_core::units::{ Duration, Energy, Length, Luminosity, Position, Temperature };
use rocketcon_app::error::{ RocketResult };
use rocketcon_core::math::{
    aggregate_power_budget,
    component_consumption_waste_heat,
    component_power_consumption,
    effective_ga_product,
    rtg_electrical_power,
    rtg_waste_heat,
    solar_panel_electrical_output,
    solar_panel_waste_heat,
    vehicle_equilibrium_temperature,
    ComponentPowerContribution,
    VehiclePowerStatus,
};
use rocketcon_core::physics_reference::RadioisotopeType;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub struct PowerSmokeTestReport {
    pub total_generation: Luminosity,
    pub total_consumption: Luminosity,
    pub net_power: Luminosity,
    pub dumped_power: Luminosity,
    pub total_stored_energy: Energy,
    pub total_battery_capacity: Energy,
    pub state_of_charge_fraction: f64,
    pub estimated_autonomy: Option<Duration>,
    pub status: VehiclePowerStatus,
    pub total_internal_waste_heat: Luminosity,
    pub effective_radiator_ga_product: f64,
    pub equilibrium_temperature: Temperature,
}

pub async fn run_mock_vehicle_power_smoke_test(
    ctx: &AppContext,
    planet_id: Uuid
) -> RocketResult<PowerSmokeTestReport> {
    let universe_epoch = rocketcon_app::universe::resolve_universe_epoch(ctx.pool()).await?;
    let at_epoch = Duration::new(0.0);

    let snapshot = rocketcon_app::environment::load_environment_snapshot(
        ctx.pool(),
        planet_id,
        universe_epoch,
        at_epoch
    ).await?;

    let radius = snapshot.planet.equatorial_radius().unwrap_or_else(|| Length::new(6_371_000.0));

    let vehicle_altitude_m = 400_000.0;
    let vehicle_position =
        snapshot.planet_position +
        Position::from_components(radius.value() + vehicle_altitude_m, 0.0, 0.0);

    let irradiance = resolve_irradiance_at_position(
        ctx.pool(),
        &snapshot.star,
        universe_epoch,
        at_epoch,
        vehicle_position
    ).await?;

    let is_eclipsed = is_in_cylindrical_shadow(
        vehicle_position,
        snapshot.star_position,
        snapshot.planet_position,
        radius
    );

    let solar_area_m2 = 8.0;
    let solar_efficiency = 0.28;
    let solar_absorptivity = 0.9;
    let solar_is_tracking = true;

    let solar_gen = solar_panel_electrical_output(
        irradiance,
        solar_area_m2,
        solar_efficiency,
        solar_is_tracking,
        is_eclipsed
    );
    let solar_waste = solar_panel_waste_heat(
        irradiance,
        solar_area_m2,
        solar_absorptivity,
        solar_efficiency,
        solar_is_tracking,
        is_eclipsed
    );
    let solar_contrib = ComponentPowerContribution::new(
        solar_gen,
        Luminosity::new(0.0),
        solar_waste
    );

    let rtg_fuel_mass = astronomicon_core::units::Mass::new(4.5);
    let rtg_efficiency = 0.065;
    let rtg_elapsed = Duration::new(5.0 * 365.25 * 86_400.0);
    let rtg_gen = rtg_electrical_power(
        RadioisotopeType::Plutonium238,
        rtg_fuel_mass,
        rtg_efficiency,
        rtg_elapsed
    );
    let rtg_waste = rtg_waste_heat(
        RadioisotopeType::Plutonium238,
        rtg_fuel_mass,
        rtg_efficiency,
        rtg_elapsed
    );
    let rtg_contrib = ComponentPowerContribution::new(rtg_gen, Luminosity::new(0.0), rtg_waste);

    let avionics_rated = Luminosity::new(120.0);
    let avionics_load = 1.0;
    let avionics_con = component_power_consumption(avionics_rated, avionics_load);
    let avionics_waste = component_consumption_waste_heat(avionics_con);
    let avionics_contrib = ComponentPowerContribution::new(
        Luminosity::new(0.0),
        avionics_con,
        avionics_waste
    );

    let payload_rated = Luminosity::new(350.0);
    let payload_load = 0.8;
    let payload_con = component_power_consumption(payload_rated, payload_load);
    let payload_waste = component_consumption_waste_heat(payload_con);
    let payload_contrib = ComponentPowerContribution::new(
        Luminosity::new(0.0),
        payload_con,
        payload_waste
    );

    let comms_rated = Luminosity::new(80.0);
    let comms_load = 0.5;
    let comms_con = component_power_consumption(comms_rated, comms_load);
    let comms_waste = component_consumption_waste_heat(comms_con);
    let comms_contrib = ComponentPowerContribution::new(
        Luminosity::new(0.0),
        comms_con,
        comms_waste
    );

    let contributions = [
        solar_contrib,
        rtg_contrib,
        avionics_contrib,
        payload_contrib,
        comms_contrib,
    ];

    let battery_capacity = Energy::new(10_000_000.0);
    let battery_stored = Energy::new(8_500_000.0);
    let battery_max_charge_power = Luminosity::new(1_500.0);

    let total_gen_val = solar_gen.value() + rtg_gen.value();
    let total_con_val = avionics_con.value() + payload_con.value() + comms_con.value();
    let net_power_val = total_gen_val - total_con_val;

    let dumped_power = if net_power_val > 0.0 {
        if battery_stored.value() >= battery_capacity.value() {
            Luminosity::new(net_power_val)
        } else if net_power_val > battery_max_charge_power.value() {
            Luminosity::new(net_power_val - battery_max_charge_power.value())
        } else {
            Luminosity::new(0.0)
        }
    } else {
        Luminosity::new(0.0)
    };

    let budget = aggregate_power_budget(
        &contributions,
        battery_capacity,
        battery_stored,
        dumped_power
    );

    let radiator_specs = [
        (3.5, 0.88),
        (2.0, 0.85),
    ];
    let effective_ga = effective_ga_product(&radiator_specs);
    let eq_temp = vehicle_equilibrium_temperature(budget.total_internal_waste_heat, effective_ga);

    let report = PowerSmokeTestReport {
        total_generation: budget.total_generation,
        total_consumption: budget.total_consumption,
        net_power: budget.net_power,
        dumped_power: budget.dumped_power,
        total_stored_energy: budget.total_stored_energy,
        total_battery_capacity: budget.total_battery_capacity,
        state_of_charge_fraction: budget.state_of_charge_fraction,
        estimated_autonomy: budget.estimated_autonomy,
        status: budget.status,
        total_internal_waste_heat: budget.total_internal_waste_heat,
        effective_radiator_ga_product: effective_ga,
        equilibrium_temperature: eq_temp,
    };

    println!("Rocketcon Mock Vehicle Power Smoke Test Report:");
    println!("  Solar Irradiance: {:.2} W/m² (Eclipsed: {})", irradiance.value(), is_eclipsed);
    println!("  Total Generation: {:.2} W", report.total_generation.value());
    println!("  Total Consumption: {:.2} W", report.total_consumption.value());
    println!("  Net Power: {:.2} W", report.net_power.value());
    println!("  Dumped Power: {:.2} W", report.dumped_power.value());
    println!(
        "  Stored Energy: {:.2} / {:.2} MJ",
        report.total_stored_energy.value() / 1e6,
        report.total_battery_capacity.value() / 1e6
    );
    println!("  State of Charge: {:.2} %", report.state_of_charge_fraction * 100.0);
    match report.estimated_autonomy {
        Some(autonomy) => {
            println!("  Estimated Autonomy: {:.2} s", autonomy.value());
        }
        None => println!("  Estimated Autonomy: Infinite / Charging"),
    }
    println!("  Power Status: {:?}", report.status);
    println!("  Total Internal Waste Heat: {:.2} W", report.total_internal_waste_heat.value());
    println!("  Effective Radiator GA: {:.3} m²", report.effective_radiator_ga_product);
    println!(
        "  Equilibrium Temperature: {:.2} K ({:.2} °C)",
        report.equilibrium_temperature.value(),
        report.equilibrium_temperature.value() - 273.15
    );

    Ok(report)
}
