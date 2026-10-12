use astronomicon_core::domain::{Planet, Star};
use rocketcon_core::math::orbital::sphere_of_influence::laplace_sphere_of_influence_radius;

pub fn soi_orbit_state(scenario: &str, planet: &Planet, star: &Star)
    -> ([f64; 3], [f64; 3]) {
    let radius = laplace_sphere_of_influence_radius(
        planet.orbital_elements().unwrap().semi_major_axis(),
        planet.mass(), star.mass()).value();
    let (distance, radial_velocity, tangent_velocity) = match scenario {
        "--c9-soi-outbound" => (radius - 1_500_000.0, 6_000.0, 300.0),
        "--c10-soi-outbound" => (radius - 15_000.0, 6_000.0, 300.0),
        "--c10-soi-inbound" => (radius + 15_000.0, -6_000.0, 300.0),
        "--c10-soi-tangent" => (radius + 15_000.0, 0.0, 300.0),
        "--c9-soi-inbound" => (radius - 1_500_000.0, -4_000.0, 300.0),
        "--c9-soi-inbound-stable" => (radius - 1_500_000.0, -200.0, 300.0),
        "--c9-soi-inbound-nearcircular" => (radius - 1_500_000.0, -40.0, 500.0),
        "--c9-soi-outside" => (radius + 1_500_000.0, -2_000.0, 300.0),
        _ => unreachable!(),
    };
    ([distance, 0.0, 0.0], [radial_velocity, tangent_velocity, 0.0])
}
