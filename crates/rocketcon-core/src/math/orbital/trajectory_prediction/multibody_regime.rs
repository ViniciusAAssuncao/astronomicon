use crate::constants::{
    HILL_SPHERE_MULTI_BODY_FRACTION, LAGRANGE_ZONE_RADIUS_RATIO,
    N_BODY_PERTURBATION_RATIO_THRESHOLD,
};
use crate::math::orbital::n_body::{
    perturbation_to_primary_ratio, CowellPerturbationConfig, NBodySystemBody,
};
use crate::math::orbital::sphere_of_influence::CelestialBodySoi;
use astronomicon_core::units::{Position, VelocityVector};

pub fn build_cowell_config(
    current_body: &CelestialBodySoi,
    candidate_soi_bodies: &[CelestialBodySoi],
) -> CowellPerturbationConfig {
    let primary_sys = NBodySystemBody::new(
        current_body.id(),
        current_body.mass(),
        current_body.position(),
        VelocityVector::zero(),
        current_body.body_radius(),
        None,
    );

    let mut perturbing = Vec::new();
    for body in candidate_soi_bodies {
        if body.id() != current_body.id() {
            perturbing.push(NBodySystemBody::new(
                body.id(),
                body.mass(),
                body.position(),
                VelocityVector::zero(),
                body.body_radius(),
                None,
            ));
        }
    }

    CowellPerturbationConfig {
        primary_body: primary_sys,
        perturbing_bodies: perturbing,
    }
}

pub fn is_in_multibody_regime(
    position_rel_primary: Position,
    current_body: &CelestialBodySoi,
    candidate_soi_bodies: &[CelestialBodySoi],
) -> bool {
    let r_mag = position_rel_primary.raw().magnitude();
    let r_soi = current_body.soi_radius().value();

    if r_soi.is_finite() && r_mag > r_soi * HILL_SPHERE_MULTI_BODY_FRACTION {
        return true;
    }

    let cowell_config = build_cowell_config(current_body, candidate_soi_bodies);
    let pert_ratio = perturbation_to_primary_ratio(position_rel_primary.raw(), &cowell_config);
    if pert_ratio > N_BODY_PERTURBATION_RATIO_THRESHOLD {
        return true;
    }

    for body in candidate_soi_bodies {
        if body.id() != current_body.id() {
            let dist_to_body = (position_rel_primary.raw() - body.position().raw()).magnitude();
            if dist_to_body < body.soi_radius().value() * LAGRANGE_ZONE_RADIUS_RATIO {
                return true;
            }
        }
    }

    false
}