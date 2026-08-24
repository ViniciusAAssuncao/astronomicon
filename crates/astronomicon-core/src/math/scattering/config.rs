use crate::units::Length;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MultipleScatteringConfig {
    pub view_samples: u32,
    pub sun_samples: u32,
    pub atmosphere_top_altitude: Length,
    pub ground_albedo: f64,
    pub multiple_scattering_factor: f64,
}

impl MultipleScatteringConfig {
    pub fn new(
        view_samples: u32,
        sun_samples: u32,
        atmosphere_top_altitude: Length,
        ground_albedo: f64,
        multiple_scattering_factor: f64,
    ) -> Self {
        Self {
            view_samples: view_samples.max(4),
            sun_samples: sun_samples.max(2),
            atmosphere_top_altitude,
            ground_albedo: ground_albedo.clamp(0.0, 1.0),
            multiple_scattering_factor: multiple_scattering_factor.clamp(0.0, 5.0),
        }
    }

    pub fn fast() -> Self {
        Self {
            view_samples: 16,
            sun_samples: 8,
            atmosphere_top_altitude: Length::new(100_000.0),
            ground_albedo: 0.15,
            multiple_scattering_factor: 1.0,
        }
    }

    pub fn accurate() -> Self {
        Self {
            view_samples: 64,
            sun_samples: 32,
            atmosphere_top_altitude: Length::new(100_000.0),
            ground_albedo: 0.15,
            multiple_scattering_factor: 1.0,
        }
    }

    pub fn with_ground_albedo(mut self, ground_albedo: f64) -> Self {
        self.ground_albedo = ground_albedo.clamp(0.0, 1.0);
        self
    }

    pub fn with_multiple_scattering_factor(mut self, factor: f64) -> Self {
        self.multiple_scattering_factor = factor.clamp(0.0, 5.0);
        self
    }
}

impl Default for MultipleScatteringConfig {
    fn default() -> Self {
        Self {
            view_samples: 32,
            sun_samples: 16,
            atmosphere_top_altitude: Length::new(100_000.0),
            ground_albedo: 0.15,
            multiple_scattering_factor: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MultipleScatteringResult {
    pub single_scattered_radiance: f64,
    pub multiple_scattered_radiance: f64,
    pub total_radiance: f64,
    pub optical_depth: f64,
    pub transmittance: f64,
}
