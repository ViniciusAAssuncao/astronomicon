use astronomicon_core::units::Angle;
use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LatitudinalBand {
    pub latitude: Angle,
    pub weight: f64,
}

pub fn standard_latitude_bands(n_bands: usize) -> Vec<LatitudinalBand> {
    let n = n_bands.max(2);
    let d_phi = PI / (n as f64);
    let mut bands = Vec::with_capacity(n);
    let mut total_weight = 0.0;

    for i in 0..n {
        let phi = -PI / 2.0 + ((i as f64) + 0.5) * d_phi;
        let w = phi.cos() * d_phi * 0.5;
        total_weight += w;
        bands.push(LatitudinalBand {
            latitude: Angle::new(phi),
            weight: w,
        });
    }

    if total_weight > 0.0 {
        for band in &mut bands {
            band.weight /= total_weight;
        }
    }

    bands
}
