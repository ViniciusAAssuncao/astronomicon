use crate::math::radiation::planck_spectral_radiance;
use crate::units::constants::{ CIE_WAVELENGTH_MAX_M, CIE_WAVELENGTH_MIN_M, CIE_WAVELENGTH_STEP_M };
use crate::units::{ ColorRGB, SpectralRadiance, Temperature, Wavelength };
use serde::{ Deserialize, Serialize };

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ColorXYZ {
    x: f64,
    y: f64,
    z: f64,
}

impl ColorXYZ {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }

    pub fn x(&self) -> f64 {
        self.x
    }

    pub fn y(&self) -> f64 {
        self.y
    }

    pub fn z(&self) -> f64 {
        self.z
    }

    pub fn luminance(&self) -> f64 {
        self.y
    }
}

impl std::ops::Add for ColorXYZ {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl std::ops::Sub for ColorXYZ {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl std::ops::Mul<f64> for ColorXYZ {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl std::ops::Div<f64> for ColorXYZ {
    type Output = Self;
    fn div(self, rhs: f64) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs, self.z / rhs)
    }
}

impl std::ops::Neg for ColorXYZ {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y, -self.z)
    }
}

fn cie_gaussian_fit(wavelength_nm: f64, mu: f64, inv_sigma1: f64, inv_sigma2: f64) -> f64 {
    let inv_sigma = if wavelength_nm < mu { inv_sigma1 } else { inv_sigma2 };
    let t = (wavelength_nm - mu) * inv_sigma;
    (-0.5 * t * t).exp()
}

pub fn cie_x_bar(wavelength: Wavelength) -> f64 {
    let lambda_nm = wavelength.value() * 1.0e9;
    if !lambda_nm.is_finite() {
        return 0.0;
    }

    let t1 = 0.362 * cie_gaussian_fit(lambda_nm, 442.0, 0.0624, 0.0374);
    let t2 = 1.056 * cie_gaussian_fit(lambda_nm, 599.8, 0.0264, 0.0323);
    let t3 = 0.065 * cie_gaussian_fit(lambda_nm, 501.1, 0.049, 0.0382);

    let val = t1 + t2 - t3;
    if !val.is_finite() {
        0.0
    } else {
        val.max(0.0)
    }
}

pub fn cie_y_bar(wavelength: Wavelength) -> f64 {
    let lambda_nm = wavelength.value() * 1.0e9;
    if !lambda_nm.is_finite() {
        return 0.0;
    }

    let t1 = 0.821 * cie_gaussian_fit(lambda_nm, 568.8, 0.0213, 0.0247);
    let t2 = 0.286 * cie_gaussian_fit(lambda_nm, 530.9, 0.0613, 0.0322);

    let val = t1 + t2;
    if !val.is_finite() {
        0.0
    } else {
        val.max(0.0)
    }
}

pub fn cie_z_bar(wavelength: Wavelength) -> f64 {
    let lambda_nm = wavelength.value() * 1.0e9;
    if !lambda_nm.is_finite() {
        return 0.0;
    }

    let t1 = 1.217 * cie_gaussian_fit(lambda_nm, 437.0, 0.0845, 0.0278);
    let t2 = 0.681 * cie_gaussian_fit(lambda_nm, 459.0, 0.0385, 0.0725);

    let val = t1 + t2;
    if !val.is_finite() {
        0.0
    } else {
        val.max(0.0)
    }
}

pub fn cie_color_matching_functions(wavelength: Wavelength) -> ColorXYZ {
    ColorXYZ::new(cie_x_bar(wavelength), cie_y_bar(wavelength), cie_z_bar(wavelength))
}

pub fn spectral_radiance_to_xyz<F>(spectrum: F) -> ColorXYZ
    where F: Fn(Wavelength) -> SpectralRadiance
{
    let step = CIE_WAVELENGTH_STEP_M;
    if step <= 0.0 || !step.is_finite() {
        return ColorXYZ::zero();
    }

    let mut accumulated = ColorXYZ::zero();
    let mut lambda_m = CIE_WAVELENGTH_MIN_M;

    while lambda_m <= CIE_WAVELENGTH_MAX_M {
        let wavelength = Wavelength::new(lambda_m);
        let radiance = spectrum(wavelength).value();

        if radiance.is_finite() && radiance > 0.0 {
            let cmf = cie_color_matching_functions(wavelength);
            accumulated = accumulated + cmf * radiance;
        }

        lambda_m += step;
    }

    accumulated * step
}

pub fn blackbody_spectrum_to_xyz(temperature: Temperature) -> ColorXYZ {
    spectral_radiance_to_xyz(|wavelength| {
        SpectralRadiance::new(planck_spectral_radiance(wavelength, temperature))
    })
}

pub fn chromatically_adapt_xyz(
    xyz: ColorXYZ,
    source_white: ColorXYZ,
    target_white: ColorXYZ
) -> ColorXYZ {
    let sx = if source_white.x() > 1e-12 { source_white.x() } else { 1.0 };
    let sy = if source_white.y() > 1e-12 { source_white.y() } else { 1.0 };
    let sz = if source_white.z() > 1e-12 { source_white.z() } else { 1.0 };

    let tx = target_white.x();
    let ty = target_white.y();
    let tz = target_white.z();

    ColorXYZ::new(
        xyz.x() * (tx / sx) * (sy / ty),
        xyz.y(),
        xyz.z() * (tz / sz) * (sy / ty)
    )
}

pub fn xyz_to_linear_srgb(xyz: ColorXYZ) -> ColorRGB {
    let x = xyz.x();
    let y = xyz.y();
    let z = xyz.z();

    let r = 3.2406 * x - 1.5372 * y - 0.4986 * z;
    let g = -0.9689 * x + 1.8758 * y + 0.0415 * z;
    let b = 0.0557 * x - 0.204 * y + 1.057 * z;

    let clamp_channel = |c: f64| if c.is_finite() { c.max(0.0) } else { 0.0 };

    ColorRGB::new(clamp_channel(r), clamp_channel(g), clamp_channel(b))
}

pub fn exposure_tone_map(color: ColorRGB, exposure: f64) -> ColorRGB {
    let e = if exposure.is_finite() && exposure > 0.0 { exposure } else { 1.0 };
    let lum = color.luminance();
    if !lum.is_finite() || lum <= 0.0 {
        return ColorRGB::zero();
    }
    let mapped_lum = 1.0 - (-lum * e).exp();
    let scale = mapped_lum / lum;
    ColorRGB::new(
        (color.r() * scale).max(0.0),
        (color.g() * scale).max(0.0),
        (color.b() * scale).max(0.0)
    )
}

pub fn reinhard_tone_map(color: ColorRGB) -> ColorRGB {
    let lum = color.luminance();
    if !lum.is_finite() || lum <= 0.0 {
        return ColorRGB::zero();
    }
    let mapped_lum = lum / (1.0 + lum);
    let scale = mapped_lum / lum;
    ColorRGB::new(
        (color.r() * scale).max(0.0),
        (color.g() * scale).max(0.0),
        (color.b() * scale).max(0.0)
    )
}

pub fn reinhard_extended_tone_map(color: ColorRGB, white_point_luminance: f64) -> ColorRGB {
    let l_white = if white_point_luminance.is_finite() && white_point_luminance > 0.0 {
        white_point_luminance
    } else {
        1.0
    };
    let l_white_sq = l_white * l_white;
    let lum = color.luminance();
    if !lum.is_finite() || lum <= 0.0 {
        return ColorRGB::zero();
    }
    let mapped_lum = (lum * (1.0 + lum / l_white_sq)) / (1.0 + lum);
    let scale = mapped_lum / lum;
    ColorRGB::new(
        (color.r() * scale).max(0.0),
        (color.g() * scale).max(0.0),
        (color.b() * scale).max(0.0)
    )
}

pub fn linear_to_srgb_gamma(color: ColorRGB) -> ColorRGB {
    let encode = |c: f64| {
        let clamped = c.clamp(0.0, 1.0);
        if clamped <= 0.0031308 {
            12.92 * clamped
        } else {
            1.055 * clamped.powf(1.0 / 2.4) - 0.055
        }
    };

    ColorRGB::new(encode(color.r()), encode(color.g()), encode(color.b()))
}

pub fn stellar_temperature_to_display_rgb(temperature: Temperature, exposure: f64) -> ColorRGB {
    let xyz = blackbody_spectrum_to_xyz(temperature);
    let linear_rgb = xyz_to_linear_srgb(xyz);
    let exposed = exposure_tone_map(linear_rgb, exposure);
    linear_to_srgb_gamma(exposed)
}