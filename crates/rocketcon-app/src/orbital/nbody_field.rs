use crate::error::{RocketError, RocketResult};
use astronomicon_app::ephemeris::EphemerisCatalog;
use astronomicon_core::units::constants::GRAVITATIONAL_CONSTANT;
use astronomicon_core::units::{AccelerationVector, Duration, Vector3};
use rocketcon_core::math::orbital::n_body::dop853::{
    Dop853Config, Dop853Error, GravitySource, inertial_point_mass_derivative_from_sources,
};
use rocketcon_core::math::orbital::n_body::impact::ImpactCandidate;
use uuid::Uuid;

pub(crate) struct CatalogGravityField<'a> {
    catalog: &'a EphemerisCatalog,
    epoch: Duration,
    origin: Vector3,
    ids: Vec<Uuid>,
    sources: Vec<GravitySource>,
    candidates: Vec<ImpactCandidate>,
}

impl<'a> CatalogGravityField<'a> {
    pub(crate) fn new(
        catalog: &'a EphemerisCatalog,
        epoch: Duration,
        origin: Vector3,
        selected_ids: Option<&[Uuid]>,
    ) -> RocketResult<Self> {
        if ![origin.0, origin.1, origin.2, epoch.value()]
            .iter()
            .all(|v| v.is_finite())
        {
            return Err(RocketError::Generic(
                "invalid N-body origin or epoch".into(),
            ));
        }
        let mut bodies = catalog
            .gravitating_bodies()
            .into_iter()
            .map(|(body_id, mass)| {
                let mu = mass.value() * GRAVITATIONAL_CONSTANT;
                let radius = catalog.radius_of_body(body_id);
                if !mu.is_finite() || mu <= 0.0 {
                    return Err(RocketError::Generic(format!(
                        "invalid gravitational parameter for {body_id}"
                    )));
                }
                Ok((
                    GravitySource {
                        body_id,
                        gravitational_parameter_m3_s2: mu,
                        position_m: [0.0; 3],
                    },
                    radius,
                ))
            })
            .collect::<RocketResult<Vec<_>>>()?;
        if bodies.is_empty() {
            return Err(RocketError::Generic(
                "N-body catalog has no massive bodies".into(),
            ));
        }
        bodies.sort_by_key(|(source, _)| source.body_id);
        let candidates = bodies
            .iter()
            .filter_map(|(source, radius)| {
                radius.map(|radius_m| ImpactCandidate {
                    body_id: source.body_id,
                    radius_m,
                })
            })
            .collect();
        let mut sources = bodies
            .into_iter()
            .map(|(source, _)| source)
            .collect::<Vec<_>>();
        if let Some(selected) = selected_ids {
            let mut sorted = selected.to_vec();
            sorted.sort();
            sorted.dedup();
            if sorted.len() != selected.len()
                || sorted.is_empty()
                || sorted.iter().any(|id| {
                    sources
                        .binary_search_by_key(id, |source| source.body_id)
                        .is_err()
                })
            {
                return Err(RocketError::Generic(
                    "invalid selected N-body source set".into(),
                ));
            }
            sources.retain(|source| sorted.binary_search(&source.body_id).is_ok());
        }
        let ids = sources.iter().map(|source| source.body_id).collect();
        Ok(Self {
            catalog,
            epoch,
            origin,
            ids,
            sources,
            candidates,
        })
    }

    pub(crate) fn impact_candidates(&self) -> Vec<ImpactCandidate> {
        self.candidates.clone()
    }

    pub(crate) fn acceleration_at(
        &mut self,
        elapsed_seconds: f64,
        local_position: Vector3,
    ) -> Result<AccelerationVector, Dop853Error> {
        let positions = self
            .catalog
            .positions_for_ids_at_offset(self.epoch, elapsed_seconds, &self.ids)
            .map_err(|error| Dop853Error::Derivative(error.to_string()))?;
        for source in &mut self.sources {
            let world = positions
                .get(&source.body_id)
                .ok_or_else(|| {
                    Dop853Error::Derivative(format!(
                        "N-body body {} has no ephemeris",
                        source.body_id
                    ))
                })?
                .raw();
            let relative = world - self.origin;
            let delta = relative - local_position;
            let distance_squared = delta.dot(&delta);
            if !distance_squared.is_finite() || distance_squared <= 0.0 {
                return Err(Dop853Error::GravitationalSingularity);
            }
            source.position_m = [relative.0, relative.1, relative.2];
        }
        let y = [
            local_position.0,
            local_position.1,
            local_position.2,
            0.0,
            0.0,
            0.0,
        ];
        let derivative = inertial_point_mass_derivative_from_sources(&y, &self.sources)?;
        Ok(AccelerationVector::from_raw(Vector3::new(
            derivative[3],
            derivative[4],
            derivative[5],
        )))
    }
}

pub(crate) fn tick_policy<const N: usize>(
    dt: Duration,
    absolute_tolerances: [f64; N],
) -> RocketResult<Dop853Config<N>> {
    let seconds = dt.value();
    if !seconds.is_finite() || seconds <= 0.0 {
        return Err(RocketError::Generic(
            "N-body integration requires a positive finite timestep".into(),
        ));
    }
    Ok(Dop853Config {
        absolute_tolerances,
        relative_tolerance: 1e-10,
        initial_step_s: seconds * 0.25,
        minimum_step_s: seconds * 1e-8,
        maximum_step_s: seconds,
        maximum_attempts: 128,
        maximum_evaluations: 2048,
        maximum_segments: 128,
    })
}
