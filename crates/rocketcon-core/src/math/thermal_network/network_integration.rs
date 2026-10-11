use super::radiative_exchange::{node_net_radiation, node_solar_heat_gain};
use super::types::{ThermalNetworkDerivative, ThermalNetworkState};
use astronomicon_core::units::{Duration, Irradiance, Temperature, Vector3};

pub fn compute_thermal_network_derivatives(
    network: &ThermalNetworkState,
    environment_temperature: Temperature,
    solar_irradiance: Irradiance,
    sun_direction_body: Vector3,
    is_eclipsed: bool,
) -> ThermalNetworkDerivative {
    let n = network.nodes.len();
    if n == 0 {
        return ThermalNetworkDerivative::zero(0);
    }

    let mut q_net = vec![0.0; n];

    for i in 0..n {
        let node = &network.nodes[i];
        let q_gen = node.internal_heat_generation.value();
        let q_aero = node.external_aerodynamic_heat.value();
        let q_solar = node_solar_heat_gain(
            node,
            solar_irradiance,
            sun_direction_body,
            is_eclipsed,
        )
        .value();
        let q_rad_out = node_net_radiation(node, environment_temperature).value();

        q_net[i] += q_gen + q_aero + q_solar - q_rad_out;
    }

    for edge in &network.edges {
        let ta = network.nodes[edge.node_a].temperature.value();
        let tb = network.nodes[edge.node_b].temperature.value();
        let q_flow = edge.conductance_w_per_k * (tb - ta);

        q_net[edge.node_a] += q_flow;
        q_net[edge.node_b] -= q_flow;
    }

    let mut d_temps = vec![0.0; n];
    for i in 0..n {
        let cap = network.nodes[i].thermal_capacitance.value();
        if cap > 0.0 && cap.is_finite() {
            d_temps[i] = q_net[i] / cap;
        }
    }

    ThermalNetworkDerivative::new(d_temps)
}

pub fn thermal_network_rk4_step(
    network: &ThermalNetworkState,
    environment_temperature: Temperature,
    solar_irradiance: Irradiance,
    sun_direction_body: Vector3,
    is_eclipsed: bool,
    dt: Duration,
) -> ThermalNetworkState {
    let h = dt.value();
    if h <= 0.0 || !h.is_finite() || network.nodes.is_empty() {
        return network.clone();
    }

    let n = network.nodes.len();
    let half_h = 0.5 * h;
    let sixth_h = h / 6.0;

    let t0 = network.temperatures();

    let d1 = compute_thermal_network_derivatives(
        network,
        environment_temperature,
        solar_irradiance,
        sun_direction_body,
        is_eclipsed,
    );

    let mut s1 = network.clone();
    let mut t_s1 = Vec::with_capacity(n);
    for i in 0..n {
        t_s1.push(Temperature::new((t0[i].value() + d1.d_temperatures[i] * half_h).max(0.0)));
    }
    s1.set_temperatures(&t_s1);

    let d2 = compute_thermal_network_derivatives(
        &s1,
        environment_temperature,
        solar_irradiance,
        sun_direction_body,
        is_eclipsed,
    );

    let mut s2 = network.clone();
    let mut t_s2 = Vec::with_capacity(n);
    for i in 0..n {
        t_s2.push(Temperature::new((t0[i].value() + d2.d_temperatures[i] * half_h).max(0.0)));
    }
    s2.set_temperatures(&t_s2);

    let d3 = compute_thermal_network_derivatives(
        &s2,
        environment_temperature,
        solar_irradiance,
        sun_direction_body,
        is_eclipsed,
    );

    let mut s3 = network.clone();
    let mut t_s3 = Vec::with_capacity(n);
    for i in 0..n {
        t_s3.push(Temperature::new((t0[i].value() + d3.d_temperatures[i] * h).max(0.0)));
    }
    s3.set_temperatures(&t_s3);

    let d4 = compute_thermal_network_derivatives(
        &s3,
        environment_temperature,
        solar_irradiance,
        sun_direction_body,
        is_eclipsed,
    );

    let mut final_network = network.clone();
    let mut final_temps = Vec::with_capacity(n);
    for i in 0..n {
        let delta = (d1.d_temperatures[i]
            + 2.0 * d2.d_temperatures[i]
            + 2.0 * d3.d_temperatures[i]
            + d4.d_temperatures[i])
            * sixth_h;
        final_temps.push(Temperature::new((t0[i].value() + delta).max(0.0)));
    }
    final_network.set_temperatures(&final_temps);

    final_network
}

pub fn integrate_thermal_network(
    network: &ThermalNetworkState,
    environment_temperature: Temperature,
    solar_irradiance: Irradiance,
    sun_direction_body: Vector3,
    is_eclipsed: bool,
    total_duration: Duration,
    substep_duration: Duration,
) -> ThermalNetworkState {
    let total_dt = total_duration.value();
    let sub_dt = substep_duration.value();

    if total_dt <= 0.0 || !total_dt.is_finite() || network.nodes.is_empty() {
        return network.clone();
    }

    if sub_dt <= 0.0 || !sub_dt.is_finite() || sub_dt >= total_dt {
        return thermal_network_rk4_step(
            network,
            environment_temperature,
            solar_irradiance,
            sun_direction_body,
            is_eclipsed,
            total_duration,
        );
    }

    let mut current = network.clone();
    let mut remaining = total_dt;

    while remaining > 1e-9 {
        let step = remaining.min(sub_dt);
        current = thermal_network_rk4_step(
            &current,
            environment_temperature,
            solar_irradiance,
            sun_direction_body,
            is_eclipsed,
            Duration::new(step),
        );
        remaining -= step;
    }

    current
}
