use crate::constants::{
    ATMOSPHERIC_PASS_DEFAULT_DT_S, CHEBYSHEV_DEFAULT_FIT_DEGREE,
    DEFAULT_AEROCAPTURE_DRAG_COEFFICIENT, DEFAULT_AEROCAPTURE_LIFT_TO_DRAG_RATIO,
    DEFAULT_AEROCAPTURE_MAX_G_LOAD, DEFAULT_AEROCAPTURE_MAX_HEAT_FLUX_W_PER_M2,
    DEFAULT_AEROCAPTURE_NOSE_RADIUS_M, DEFAULT_AEROCAPTURE_PASS_FALLBACK_DURATION_S,
    DEFAULT_AEROCAPTURE_PASS_MAX_DURATION_S, DEFAULT_AEROCAPTURE_REFERENCE_AREA_M2,
    DEFAULT_AEROCAPTURE_VEHICLE_MASS_KG, DEFAULT_ATMOSPHERE_SCALE_HEIGHT_M,
    DEFAULT_ATMOSPHERE_SURFACE_DENSITY_KG_PER_M3,
};
use crate::domain::{ConicPatchData, TrajectoryPatch, TrajectoryPatchKind};
use crate::error::RocketDomainResult;
use crate::math::aerothermodynamics::aerocapture::{
    simulate_atmospheric_pass, AerocaptureOutcome, AerocaptureVehicleProperties,
    AtmosphericModelParameters,
};
use crate::math::orbital::sphere_of_influence::CelestialBodySoi;
use crate::math::orbital::universal::propagate_universal_state_vectors;
use crate::math::orbital::OsculatingElements;
use astronomicon_core::units::{
    Density, Duration, GravitationalParameter, HeatFlux, Length, Mass, Position, VelocityVector,
};
use uuid::Uuid;

pub fn default_aerocapture_vehicle_properties() -> AerocaptureVehicleProperties {
    AerocaptureVehicleProperties::new(
        Mass::new(DEFAULT_AEROCAPTURE_VEHICLE_MASS_KG),
        DEFAULT_AEROCAPTURE_REFERENCE_AREA_M2,
        DEFAULT_AEROCAPTURE_DRAG_COEFFICIENT,
        DEFAULT_AEROCAPTURE_LIFT_TO_DRAG_RATIO,
        Length::new(DEFAULT_AEROCAPTURE_NOSE_RADIUS_M),
        HeatFlux::new(DEFAULT_AEROCAPTURE_MAX_HEAT_FLUX_W_PER_M2),
        DEFAULT_AEROCAPTURE_MAX_G_LOAD,
    )
}

pub fn handle_atmospheric_entry_transition(
    vehicle_id: Uuid,
    curr_state: (Position, VelocityVector),
    curr_body: &CelestialBodySoi,
    curr_mu: GravitationalParameter,
    curr_epoch: Duration,
    elements: &OsculatingElements,
    dt_entry: Duration,
    r_entry: Length,
    vehicle_props: &AerocaptureVehicleProperties,
) -> RocketDomainResult<Option<(Vec<TrajectoryPatch>, bool)>> {
    let mut generated_patches = Vec::new();
    let entry_epoch = curr_epoch + dt_entry;
    let pre_entry_patch = TrajectoryPatch::from_osculating_elements(
        Uuid::new_v4(),
        vehicle_id,
        curr_body.id(),
        curr_epoch,
        Some(entry_epoch),
        elements,
        curr_mu,
    )?;
    generated_patches.push(pre_entry_patch);

    let (r_entry_pos, v_entry_vel) =
        propagate_universal_state_vectors(curr_state.0, curr_state.1, curr_mu, dt_entry)?;

    let atm_params = AtmosphericModelParameters::new(
        curr_body
            .atmosphere_surface_density
            .unwrap_or_else(|| Density::new(DEFAULT_ATMOSPHERE_SURFACE_DENSITY_KG_PER_M3)),
        curr_body
            .atmosphere_scale_height
            .unwrap_or_else(|| Length::new(DEFAULT_ATMOSPHERE_SCALE_HEIGHT_M)),
        r_entry,
        curr_body.body_radius,
        curr_mu,
        None,
    );

    let pass_res = simulate_atmospheric_pass(
        r_entry_pos,
        v_entry_vel,
        entry_epoch,
        &atm_params,
        vehicle_props,
        Duration::new(DEFAULT_AEROCAPTURE_PASS_MAX_DURATION_S),
        Duration::new(ATMOSPHERIC_PASS_DEFAULT_DT_S),
    )?;

    if let Ok(chebyshev_data) = pass_res.to_low_thrust_patch_data(CHEBYSHEV_DEFAULT_FIT_DEGREE) {
        let exit_t = pass_res.exit_epoch.unwrap_or(
            entry_epoch + Duration::new(DEFAULT_AEROCAPTURE_PASS_FALLBACK_DURATION_S),
        );
        let aero_patch = TrajectoryPatch::new_low_thrust(
            Uuid::new_v4(),
            vehicle_id,
            curr_body.id(),
            entry_epoch,
            exit_t,
            curr_mu,
            chebyshev_data,
        )?;
        generated_patches.push(aero_patch);
    }

    match pass_res.outcome {
        AerocaptureOutcome::Captured {
            post_pass_elements,
            exit_epoch,
        }
        | AerocaptureOutcome::Escaped {
            exit_elements: post_pass_elements,
            exit_epoch,
        } => {
            let conic_data = ConicPatchData::new(
                post_pass_elements.semi_major_axis(),
                post_pass_elements.eccentricity(),
                post_pass_elements.inclination(),
                post_pass_elements.longitude_of_ascending_node(),
                post_pass_elements.argument_of_periapsis(),
                post_pass_elements.true_anomaly(),
            )?;
            let post_patch = TrajectoryPatch::new_with_kind(
                Uuid::new_v4(),
                vehicle_id,
                curr_body.id(),
                exit_epoch,
                None,
                curr_mu,
                TrajectoryPatchKind::Conic(conic_data),
            )?;
            generated_patches.push(post_patch);
            Ok(Some((generated_patches, true)))
        }
        _ => Ok(Some((generated_patches, true))),
    }
}