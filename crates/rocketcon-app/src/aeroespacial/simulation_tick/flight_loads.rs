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

#[cfg(test)]
mod tests {
    use super::*;
    use astronomicon_core::units::{
        AngularVelocityVector, Length, Position, Quaternion, VelocityVector,
    };
    use uuid::Uuid;

    fn state(velocity: Vector3) -> VehiclePhysicalState {
        VehiclePhysicalState::new(
            Uuid::from_u128(1),
            Position::from_raw(Vector3::zero()),
            VelocityVector::from_raw(velocity),
            Quaternion::identity(),
            AngularVelocityVector::from_raw(Vector3::zero()),
            Uuid::from_u128(2),
            Duration::new(1.0),
            Duration::new(1.0),
        )
        .unwrap()
    }

    fn contact(has_contact: bool, normal: Vector3) -> SurfaceContactState {
        SurfaceContactState::new(
            has_contact,
            Length::new(0.0),
            normal,
            VelocityVector::from_raw(Vector3::zero()),
        )
    }

    #[test]
    fn force_integration_reports_gravity_subtracted_body_loads() {
        let loads = resolve_flight_loads(
            &state(Vector3::zero()),
            &state(Vector3::new(3.0, 4.0, 5.0)),
            AccelerationVector::from_raw(Vector3::new(0.0, 0.0, 1.0)),
            &contact(false, Vector3::zero()),
            Duration::new(1.0),
            true,
        );
        assert!((loads.axial_g - 4.0 / STANDARD_GRAVITY).abs() < 1e-12);
        assert!((loads.lateral_g - 5.0 / STANDARD_GRAVITY).abs() < 1e-12);
        assert!((loads.total_g - 41.0_f64.sqrt() / STANDARD_GRAVITY).abs() < 1e-12);
    }

    #[test]
    fn force_free_coasting_reports_zero_proper_loads() {
        let loads = resolve_flight_loads(
            &state(Vector3::zero()),
            &state(Vector3::new(0.0, 0.0, 12.0)),
            AccelerationVector::from_raw(Vector3::new(0.0, 0.0, 2.0)),
            &contact(false, Vector3::zero()),
            Duration::new(1.0),
            false,
        );
        assert_eq!(
            (loads.axial_g, loads.lateral_g, loads.total_g),
            (0.0, 0.0, 0.0)
        );
    }

    #[test]
    fn surface_support_load_cancels_inward_gravity() {
        let loads = resolve_flight_loads(
            &state(Vector3::zero()),
            &state(Vector3::zero()),
            AccelerationVector::from_raw(Vector3::new(0.0, 0.0, -STANDARD_GRAVITY)),
            &contact(true, Vector3::new(0.0, 0.0, 1.0)),
            Duration::new(0.0),
            false,
        );
        assert_eq!(
            (loads.axial_g, loads.lateral_g, loads.total_g),
            (1.0, 0.0, 1.0)
        );
    }
}
