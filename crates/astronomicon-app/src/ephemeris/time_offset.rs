use astronomicon_core::domain::OrbitalElements;
use astronomicon_core::error::DomainResult;
use astronomicon_core::math::kepler::{
    orbital_position_secular, orbital_position_secular_at_offset,
};
use astronomicon_core::math::perturbation::SecularPrecessionRates;
use astronomicon_core::units::{Duration, GravitationalParameter, Position};

#[derive(Clone, Copy)]
pub struct OrbitalTime {
    pub base: Duration,
    pub offset_seconds: f64,
}

impl OrbitalTime {
    pub fn at(base: Duration, offset_seconds: f64) -> Self {
        Self {
            base,
            offset_seconds,
        }
    }

    pub fn position(
        self,
        elements: &OrbitalElements,
        mu: GravitationalParameter,
        rates: &SecularPrecessionRates,
    ) -> DomainResult<Position> {
        if self.offset_seconds == 0.0 {
            orbital_position_secular(elements, mu, rates, self.base)
        } else {
            orbital_position_secular_at_offset(elements, mu, rates, self.base, self.offset_seconds)
        }
    }
}
