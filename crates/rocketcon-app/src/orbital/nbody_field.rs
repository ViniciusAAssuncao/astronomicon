use crate::error::{RocketError, RocketResult};
use astronomicon_app::ephemeris::EphemerisCatalog;
use astronomicon_core::units::constants::GRAVITATIONAL_CONSTANT;
use astronomicon_core::units::{AccelerationVector, Duration, Vector3};
use rocketcon_core::math::orbital::n_body::dop853::{
    Dop853Config, Dop853Error, GravitySource, inertial_point_mass_derivative_from_sources,
};
use uuid::Uuid;

pub(crate) struct CatalogGravityField<'a> {
    catalog: &'a EphemerisCatalog,
    epoch: Duration,
    origin: Vector3,
    ids: Vec<Uuid>,
    sources: Vec<GravitySource>,
    radii: Vec<Option<f64>>,
}

impl<'a> CatalogGravityField<'a> {
    pub(crate) fn new(
        catalog: &'a EphemerisCatalog,
        epoch: Duration,
        origin: Vector3,
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
        let (sources, radii): (Vec<_>, Vec<_>) = bodies.into_iter().unzip();
        let ids = sources.iter().map(|source| source.body_id).collect();
        Ok(Self {
            catalog,
            epoch,
            origin,
            ids,
            sources,
            radii,
        })
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
        for (source, radius) in self.sources.iter_mut().zip(self.radii.iter()) {
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
            if radius.is_some_and(|r| distance_squared <= r * r) {
                return Err(Dop853Error::Derivative(format!(
                    "N-body trajectory intersects body {}",
                    source.body_id
                )));
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

pub(crate) fn powered_nbody_requested() -> RocketResult<bool> {
    match std::env::var("ROCKETCON_C15_NBODY_POWERED") {
        Ok(value) if value == "1" => Ok(true),
        Ok(value) if value == "0" => Ok(false),
        Err(std::env::VarError::NotPresent) => Ok(false),
        _ => Err(RocketError::Generic(
            "ROCKETCON_C15_NBODY_POWERED accepts only 0 or 1".into(),
        )),
    }
}
