use crate::domain::OrbitalElements;
use crate::error::{DomainError, DomainResult};
use crate::math::kepler::orbital_position_secular;
use crate::math::orbital_secular_parts::orbital_position_secular_at_epoch_parts;
use crate::math::perturbation::SecularPrecessionRates;
use crate::units::{Duration, GravitationalParameter, Position, VelocityVector};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnchoredEphemerisEpoch {
    universe_epoch: Duration,
    anchor_at_epoch: Duration,
}

impl AnchoredEphemerisEpoch {
    pub fn new(universe_epoch: Duration, anchor_at_epoch: Duration) -> DomainResult<Self> {
        if !universe_epoch.value().is_finite() || !anchor_at_epoch.value().is_finite() {
            return Err(DomainError::InvalidInvariant {
                field: "ephemeris_anchor".to_string(),
                reason: "both anchor epoch components must be finite".to_string(),
            });
        }
        Ok(Self {
            universe_epoch,
            anchor_at_epoch,
        })
    }

    pub fn universe_epoch(&self) -> Duration {
        self.universe_epoch
    }

    pub fn anchor_at_epoch(&self) -> Duration {
        self.anchor_at_epoch
    }

    pub fn orbit_position(
        &self,
        elements: &OrbitalElements,
        mu: GravitationalParameter,
        rates: &SecularPrecessionRates,
        at_epoch: Duration,
    ) -> DomainResult<Position> {
        let legacy_origin = orbital_position_secular(
            elements,
            mu,
            rates,
            self.universe_epoch + self.anchor_at_epoch,
        )?;
        let compensated_origin = orbital_position_secular_at_epoch_parts(
            elements,
            mu,
            rates,
            self.universe_epoch,
            self.anchor_at_epoch,
        )?;
        let compensated_target = orbital_position_secular_at_epoch_parts(
            elements,
            mu,
            rates,
            self.universe_epoch,
            at_epoch,
        )?;
        Ok(Position::from_raw(
            legacy_origin.raw() + (compensated_target.raw() - compensated_origin.raw()),
        ))
    }

    pub fn orbit_velocity_centered(
        &self,
        elements: &OrbitalElements,
        mu: GravitationalParameter,
        rates: &SecularPrecessionRates,
        at_epoch: Duration,
        half_interval: Duration,
    ) -> DomainResult<VelocityVector> {
        if !half_interval.value().is_finite() || half_interval.value() <= 0.0 {
            return Err(DomainError::InvalidInvariant {
                field: "ephemeris_velocity_interval".to_string(),
                reason: "half interval must be positive and finite".to_string(),
            });
        }
        let earlier = self.orbit_position(elements, mu, rates, at_epoch - half_interval)?;
        let later = self.orbit_position(elements, mu, rates, at_epoch + half_interval)?;
        Ok(VelocityVector::from_raw(
            (later.raw() - earlier.raw()) / (2.0 * half_interval.value()),
        ))
    }
}

pub fn remap_inertial_state_preserving_relative(
    vehicle_position: Position,
    vehicle_velocity: VelocityVector,
    old_reference_position: Position,
    old_reference_velocity: VelocityVector,
    new_reference_position: Position,
    new_reference_velocity: VelocityVector,
) -> DomainResult<(Position, VelocityVector)> {
    let finite = |value: crate::units::Vector3| {
        value.0.is_finite() && value.1.is_finite() && value.2.is_finite()
    };
    let vectors = [
        vehicle_position.raw(),
        vehicle_velocity.raw(),
        old_reference_position.raw(),
        old_reference_velocity.raw(),
        new_reference_position.raw(),
        new_reference_velocity.raw(),
    ];
    if !vectors.into_iter().all(finite) {
        return Err(DomainError::InvalidInvariant {
            field: "inertial_frame_transition".to_string(),
            reason: "all state and reference vectors must be finite".to_string(),
        });
    }
    let mapped_position =
        new_reference_position.raw() + (vehicle_position.raw() - old_reference_position.raw());
    let mapped_velocity =
        new_reference_velocity.raw() + (vehicle_velocity.raw() - old_reference_velocity.raw());
    if !finite(mapped_position) || !finite(mapped_velocity) {
        return Err(DomainError::InvalidInvariant {
            field: "inertial_frame_transition".to_string(),
            reason: "frame transition exceeded finite coordinate range".to_string(),
        });
    }
    Ok((
        Position::from_raw(mapped_position),
        VelocityVector::from_raw(mapped_velocity),
    ))
}
