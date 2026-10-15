use crate::ephemeris::node_resolution::resolve_node_at_offset;
use crate::ephemeris::time_offset::OrbitalTime;
use astronomicon_core::domain::{Barycenter, MinorPlanet, Planet, Star};
use astronomicon_core::error::{DomainError, DomainResult};
use astronomicon_core::units::{Duration, Position};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

pub struct PositionWorkspace {
    memo: HashMap<Uuid, Position>,
    visiting: HashSet<Uuid>,
    resolved: Vec<Position>,
}

impl PositionWorkspace {
    pub fn positions(&self) -> &[Position] {
        &self.resolved
    }
}

pub struct PreparedPositionLookup<'a> {
    stars: &'a [Star],
    planets: &'a [Planet],
    barycenters: &'a [Barycenter],
    minor_planets: &'a [MinorPlanet],
    star_map: HashMap<Uuid, &'a Star>,
    planet_map: HashMap<Uuid, &'a Planet>,
    barycenter_map: HashMap<Uuid, &'a Barycenter>,
    minor_planet_map: HashMap<Uuid, &'a MinorPlanet>,
    member_of: HashMap<Uuid, Uuid>,
}

impl<'a> PreparedPositionLookup<'a> {
    pub fn new(
        stars: &'a [Star],
        planets: &'a [Planet],
        barycenters: &'a [Barycenter],
        minor_planets: &'a [MinorPlanet],
    ) -> DomainResult<Self> {
        let star_map = stars.iter().map(|body| (body.id(), body)).collect();
        let planet_map = planets.iter().map(|body| (body.id(), body)).collect();
        let barycenter_map = barycenters.iter().map(|body| (body.id(), body)).collect();
        let minor_planet_map = minor_planets.iter().map(|body| (body.id(), body)).collect();
        let mut member_of = HashMap::with_capacity(barycenters.len() * 2);
        for body in barycenters {
            let primary = body.member_primary().id();
            let secondary = body.member_secondary().id();
            if member_of.insert(primary, body.id()).is_some() {
                return Err(DomainError::InvalidInvariant {
                    field: "barycenters".to_string(),
                    reason: format!("entity '{}' is a member of multiple barycenters", primary),
                });
            }
            if member_of.insert(secondary, body.id()).is_some() {
                return Err(DomainError::InvalidInvariant {
                    field: "barycenters".to_string(),
                    reason: format!("entity '{}' is a member of multiple barycenters", secondary),
                });
            }
        }
        Ok(Self {
            stars,
            planets,
            barycenters,
            minor_planets,
            star_map,
            planet_map,
            barycenter_map,
            minor_planet_map,
            member_of,
        })
    }

    pub fn workspace(&self, selected_capacity: usize) -> PositionWorkspace {
        PositionWorkspace {
            memo: HashMap::with_capacity(
                self.stars.len()
                    + self.planets.len()
                    + self.barycenters.len()
                    + self.minor_planets.len(),
            ),
            visiting: HashSet::new(),
            resolved: Vec::with_capacity(selected_capacity),
        }
    }

    pub fn selected_at_offset_into(
        &self,
        base: Duration,
        offset_seconds: f64,
        requested: &[Uuid],
        workspace: &mut PositionWorkspace,
    ) -> DomainResult<()> {
        workspace.memo.clear();
        workspace.visiting.clear();
        workspace.resolved.clear();
        if !base.value().is_finite()
            || !offset_seconds.is_finite()
            || !(base.value() + offset_seconds).is_finite()
        {
            return Err(DomainError::InvalidInvariant {
                field: "ephemeris_time".into(),
                reason: "invalid base epoch or relative offset".into(),
            });
        }
        if requested.is_empty()
            || requested
                .iter()
                .enumerate()
                .any(|(index, id)| requested[..index].contains(id))
        {
            return Err(DomainError::InvalidInvariant {
                field: "ephemeris_body_selection".into(),
                reason: "selection must contain unique body identifiers and cannot be empty".into(),
            });
        }
        let time_since_epoch = OrbitalTime::at(base, offset_seconds);
        for id in requested {
            match self.resolve(
                *id,
                &mut workspace.memo,
                &mut workspace.visiting,
                time_since_epoch,
            ) {
                Ok(position) => workspace.resolved.push(position),
                Err(error) => {
                    workspace.resolved.clear();
                    return Err(error);
                }
            }
        }
        Ok(())
    }

    pub fn selected_at_offset(
        &self,
        base: Duration,
        offset_seconds: f64,
        requested: &[Uuid],
    ) -> DomainResult<HashMap<Uuid, Position>> {
        if !base.value().is_finite()
            || !offset_seconds.is_finite()
            || !(base.value() + offset_seconds).is_finite()
        {
            return Err(DomainError::InvalidInvariant {
                field: "ephemeris_time".into(),
                reason: "invalid base epoch or relative offset".into(),
            });
        }
        if requested.is_empty()
            || requested
                .iter()
                .enumerate()
                .any(|(index, id)| requested[..index].contains(id))
        {
            return Err(DomainError::InvalidInvariant {
                field: "ephemeris_body_selection".into(),
                reason: "selection must contain unique body identifiers and cannot be empty".into(),
            });
        }
        self.compute(base, offset_seconds, Some(requested))
    }

    pub fn compute(
        &self,
        base: Duration,
        offset_seconds: f64,
        selected: Option<&[Uuid]>,
    ) -> DomainResult<HashMap<Uuid, Position>> {
        if !base.value().is_finite() || !offset_seconds.is_finite() {
            return Err(DomainError::InvalidInvariant {
                field: "ephemeris_time".into(),
                reason: "non-finite epoch or offset".into(),
            });
        }
        let time_since_epoch = OrbitalTime::at(base, offset_seconds);
        let mut memo = HashMap::with_capacity(
            self.stars.len()
                + self.planets.len()
                + self.barycenters.len()
                + self.minor_planets.len(),
        );
        let mut visiting = HashSet::new();
        if let Some(requested) = selected {
            let mut filtered = HashMap::with_capacity(requested.len());
            for id in requested {
                let position = self.resolve(*id, &mut memo, &mut visiting, time_since_epoch)?;
                filtered.insert(*id, position);
            }
            return Ok(filtered);
        }
        for body in self.barycenters {
            self.resolve(body.id(), &mut memo, &mut visiting, time_since_epoch)?;
        }
        for body in self.stars {
            self.resolve(body.id(), &mut memo, &mut visiting, time_since_epoch)?;
        }
        for body in self.planets {
            self.resolve(body.id(), &mut memo, &mut visiting, time_since_epoch)?;
        }
        for body in self.minor_planets {
            self.resolve(body.id(), &mut memo, &mut visiting, time_since_epoch)?;
        }
        Ok(memo)
    }

    fn resolve(
        &self,
        id: Uuid,
        memo: &mut HashMap<Uuid, Position>,
        visiting: &mut HashSet<Uuid>,
        at: OrbitalTime,
    ) -> DomainResult<Position> {
        resolve_node_at_offset(
            id,
            &self.member_of,
            &self.star_map,
            &self.planet_map,
            &self.barycenter_map,
            &self.minor_planet_map,
            memo,
            visiting,
            at,
        )
    }
}
