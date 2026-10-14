use astronomicon_app::ephemeris::EphemerisCatalog;
use astronomicon_core::units::{Duration, Vector3};
use rocketcon_core::math::orbital::n_body::dop853::{Dop853DenseSegment, Dop853Error};
use rocketcon_core::math::orbital::n_body::impact::{
    ImpactCandidate, ImpactEvent, first_impact_in_segment,
};
use std::collections::HashMap;

pub(crate) struct CatalogImpactMonitor<'a> {
    catalog: &'a EphemerisCatalog,
    epoch: Duration,
    origin: Vector3,
    candidates: Vec<ImpactCandidate>,
}

impl<'a> CatalogImpactMonitor<'a> {
    pub(crate) fn new(
        catalog: &'a EphemerisCatalog,
        epoch: Duration,
        origin: Vector3,
        candidates: Vec<ImpactCandidate>,
    ) -> Self {
        Self {
            catalog,
            epoch,
            origin,
            candidates,
        }
    }

    pub(crate) fn first_in<const N: usize>(
        &self,
        segment: &Dop853DenseSegment<N>,
    ) -> Result<Option<ImpactEvent>, Dop853Error> {
        if self.candidates.is_empty() {
            return Ok(None);
        }
        let accuracy_s = (segment.end_seconds - segment.start_seconds) * 1e-7;
        let ids = self
            .candidates
            .iter()
            .map(|candidate| candidate.body_id)
            .collect::<Vec<_>>();
        let mut sampled_positions = HashMap::new();
        first_impact_in_segment(
            segment,
            &self.candidates,
            accuracy_s.max(f64::EPSILON),
            8,
            |id, t| {
                let time_key = t.to_bits();
                if !sampled_positions.contains_key(&time_key) {
                    let positions = self
                        .catalog
                        .positions_for_ids_at_offset(self.epoch, t, &ids)
                        .map_err(|error| Dop853Error::Derivative(error.to_string()))?;
                    sampled_positions.insert(time_key, positions);
                }
                let position = sampled_positions
                    .get(&time_key)
                    .and_then(|positions| positions.get(&id))
                    .ok_or_else(|| {
                        Dop853Error::Derivative(format!(
                            "impact candidate {id} missing from ephemeris"
                        ))
                    })?
                    .raw()
                    - self.origin;
                Ok([position.0, position.1, position.2])
            },
        )
    }
}
