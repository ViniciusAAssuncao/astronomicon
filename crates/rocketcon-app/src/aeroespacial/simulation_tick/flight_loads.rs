use astronomicon_core::units::constants::STANDARD_GRAVITY;
use astronomicon_core::units::{AccelerationVector, Duration, Vector3};
use rocketcon_core::domain::VehiclePhysicalState;
use rocketcon_core::math::collision::SurfaceContactState;

pub(super) struct FlightLoads {
    pub axial_g: f64,
    pub lateral_g: f64,
    pub total_g: f64,
}

pub(super) fn resolve_flight_loads(
    previous: &VehiclePhysicalState,
    current: &VehiclePhysicalState,
    grav_acc: AccelerationVector,
    surface_contact: &SurfaceContactState,
    dt: Duration,
    uses_force_integration: bool,
) -> FlightLoads {
    let dt_val = dt.value();
    let net_linear_acc = if dt_val > 0.0 && dt_val.is_finite() {
        (current.velocity().raw() - previous.velocity().raw()) / dt_val
    } else {
        Vector3::zero()
    };
    let proper_acc_world = if !uses_force_integration && !surface_contact.has_contact() {
        Vector3::zero()
    } else if surface_contact.has_contact() {
        let n = surface_contact.surface_normal_world();
        let g_proj = grav_acc.raw().dot(&n);
        if g_proj < 0.0 {
            -n * g_proj
        } else {
            -grav_acc.raw()
        }
    } else {
        net_linear_acc - grav_acc.raw()
    };
    let proper_acc_body = current
        .orientation()
        .inverse()
        .rotate_vector(proper_acc_world);
    FlightLoads {
        axial_g: proper_acc_body.2 / STANDARD_GRAVITY,
        lateral_g: (proper_acc_body.0 * proper_acc_body.0 + proper_acc_body.1 * proper_acc_body.1)
            .sqrt()
            / STANDARD_GRAVITY,
        total_g: proper_acc_body.magnitude() / STANDARD_GRAVITY,
    }
}