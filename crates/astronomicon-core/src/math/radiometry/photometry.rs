use crate::units::constants::{
    CIE_1931_V_LAMBDA_BLUE,
    CIE_1931_V_LAMBDA_GREEN,
    CIE_1931_V_LAMBDA_RED,
    MAXIMUM_LUMINOUS_EFFICACY,
};
use crate::units::{ Illuminance, Luminance, Wavelength };

pub fn cie1931_v_lambda(wavelength: Wavelength) -> f64 {
    let lambda = wavelength.value();
    if !lambda.is_finite() || lambda <= 0.0 {
        return 0.0;
    }
    let lambda_nm = lambda * 1.0e9;
    if (lambda_nm - 440.0).abs() < 1.0 {
        CIE_1931_V_LAMBDA_BLUE
    } else if (lambda_nm - 550.0).abs() < 1.0 {
        CIE_1931_V_LAMBDA_GREEN
    } else if (lambda_nm - 680.0).abs() < 1.0 {
        CIE_1931_V_LAMBDA_RED
    } else if lambda_nm < 380.0 || lambda_nm > 780.0 {
        0.0
    } else if lambda_nm <= 440.0 {
        CIE_1931_V_LAMBDA_BLUE * ((lambda_nm - 380.0) / 60.0).clamp(0.0, 1.0)
    } else if lambda_nm <= 550.0 {
        let t = (lambda_nm - 440.0) / 110.0;
        CIE_1931_V_LAMBDA_BLUE + t * (CIE_1931_V_LAMBDA_GREEN - CIE_1931_V_LAMBDA_BLUE)
    } else if lambda_nm <= 680.0 {
        let t = (lambda_nm - 550.0) / 130.0;
        CIE_1931_V_LAMBDA_GREEN + t * (CIE_1931_V_LAMBDA_RED - CIE_1931_V_LAMBDA_GREEN)
    } else {
        let t = ((780.0 - lambda_nm) / 100.0).clamp(0.0, 1.0);
        CIE_1931_V_LAMBDA_RED * t
    }
}

pub fn photopic_luminance(radiance_r: f64, radiance_g: f64, radiance_b: f64) -> Luminance {
    let r = if radiance_r.is_finite() { radiance_r.max(0.0) } else { 0.0 };
    let g = if radiance_g.is_finite() { radiance_g.max(0.0) } else { 0.0 };
    let b = if radiance_b.is_finite() { radiance_b.max(0.0) } else { 0.0 };

    let effective_radiance =
        (r * CIE_1931_V_LAMBDA_RED + g * CIE_1931_V_LAMBDA_GREEN + b * CIE_1931_V_LAMBDA_BLUE) /
        3.0;

    let lum = MAXIMUM_LUMINOUS_EFFICACY * effective_radiance;
    if !lum.is_finite() || lum < 0.0 {
        Luminance::new(0.0)
    } else {
        Luminance::new(lum)
    }
}

pub fn photopic_illuminance(
    irradiance_r: f64,
    irradiance_g: f64,
    irradiance_b: f64
) -> Illuminance {
    let r = if irradiance_r.is_finite() { irradiance_r.max(0.0) } else { 0.0 };
    let g = if irradiance_g.is_finite() { irradiance_g.max(0.0) } else { 0.0 };
    let b = if irradiance_b.is_finite() { irradiance_b.max(0.0) } else { 0.0 };

    let effective_irradiance =
        (r * CIE_1931_V_LAMBDA_RED + g * CIE_1931_V_LAMBDA_GREEN + b * CIE_1931_V_LAMBDA_BLUE) /
        3.0;

    let illum = MAXIMUM_LUMINOUS_EFFICACY * effective_irradiance;
    if !illum.is_finite() || illum < 0.0 {
        Illuminance::new(0.0)
    } else {
        Illuminance::new(illum)
    }
}

pub fn photopic_luminous_efficacy(r: f64, g: f64, b: f64) -> f64 {
    let r_clamped = if r.is_finite() { r.max(0.0) } else { 0.0 };
    let g_clamped = if g.is_finite() { g.max(0.0) } else { 0.0 };
    let b_clamped = if b.is_finite() { b.max(0.0) } else { 0.0 };
    let total = (r_clamped + g_clamped + b_clamped) / 3.0;

    if total <= 0.0 {
        return 0.0;
    }

    let eff =
        (r_clamped * CIE_1931_V_LAMBDA_RED +
            g_clamped * CIE_1931_V_LAMBDA_GREEN +
            b_clamped * CIE_1931_V_LAMBDA_BLUE) /
        (3.0 * total);

    let result = MAXIMUM_LUMINOUS_EFFICACY * eff;
    if !result.is_finite() || result < 0.0 {
        0.0
    } else {
        result
    }
}
