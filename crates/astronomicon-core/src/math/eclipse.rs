use crate::units::{Length, Position};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShadowState {
    FullLight,
    Penumbra,
    Umbra,
    Antumbra,
}

impl ShadowState {
    pub fn is_full_light(&self) -> bool {
        matches!(self, Self::FullLight)
    }

    pub fn is_penumbra(&self) -> bool {
        matches!(self, Self::Penumbra)
    }

    pub fn is_umbra(&self) -> bool {
        matches!(self, Self::Umbra)
    }

    pub fn is_antumbra(&self) -> bool {
        matches!(self, Self::Antumbra)
    }

    pub fn is_occluded(&self) -> bool {
        !matches!(self, Self::FullLight)
    }
}

pub fn is_in_cylindrical_shadow(
    point: Position,
    light_source_position: Position,
    occluder_position: Position,
    occluder_radius: Length,
) -> bool {
    let r_occ = occluder_radius.value();
    if r_occ <= 0.0 || !r_occ.is_finite() {
        return false;
    }

    let p = point.raw();
    let s = light_source_position.raw();
    let o = occluder_position.raw();

    let d_so = o - s;
    let dist_so = d_so.magnitude();
    if dist_so <= 0.0 || !dist_so.is_finite() {
        return false;
    }

    let u_so = d_so / dist_so;
    let r_op = p - o;
    let axial = r_op.dot(&u_so);

    if axial <= 0.0 || !axial.is_finite() {
        return false;
    }

    let perp_sq = (r_op.dot(&r_op) - axial * axial).max(0.0);
    if !perp_sq.is_finite() {
        return false;
    }

    perp_sq < r_occ * r_occ
}

pub fn is_in_cylindrical_shadow_multi(
    point: Position,
    light_source_position: Position,
    occluders: &[(Position, Length)],
) -> bool {
    occluders.iter().any(|&(occ_pos, occ_radius)| {
        is_in_cylindrical_shadow(point, light_source_position, occ_pos, occ_radius)
    })
}

pub fn cylindrical_shadow_coordinates(
    point: Position,
    light_source_position: Position,
    occluder_position: Position,
) -> (Length, Length) {
    let p = point.raw();
    let s = light_source_position.raw();
    let o = occluder_position.raw();

    let d_so = o - s;
    let dist_so = d_so.magnitude();
    if dist_so <= 0.0 || !dist_so.is_finite() {
        return (Length::new(0.0), Length::new(0.0));
    }

    let u_so = d_so / dist_so;
    let r_op = p - o;
    let axial = r_op.dot(&u_so);
    let perp_sq = (r_op.dot(&r_op) - axial * axial).max(0.0);

    (Length::new(axial), Length::new(perp_sq.sqrt()))
}

pub fn conical_shadow_state(
    point: Position,
    light_source_position: Position,
    light_source_radius: Length,
    occluder_position: Position,
    occluder_radius: Length,
) -> ShadowState {
    let r_s = light_source_radius.value();
    let r_o = occluder_radius.value();

    if r_s <= 0.0 || r_o <= 0.0 || !r_s.is_finite() || !r_o.is_finite() {
        return ShadowState::FullLight;
    }

    let p = point.raw();
    let s = light_source_position.raw();
    let o = occluder_position.raw();

    let v_s = s - p;
    let v_o = o - p;

    let d_s = v_s.magnitude();
    let d_o = v_o.magnitude();

    if d_s <= 0.0 || d_o <= 0.0 || !d_s.is_finite() || !d_o.is_finite() || d_o >= d_s {
        return ShadowState::FullLight;
    }

    let u_s = v_s / d_s;
    let u_o = v_o / d_o;

    let cos_sep = u_s.dot(&u_o);
    if cos_sep <= 0.0 || !cos_sep.is_finite() {
        return ShadowState::FullLight;
    }

    let theta_s = (r_s / d_s).clamp(0.0, 1.0).asin();
    let theta_o = (r_o / d_o).clamp(0.0, 1.0).asin();
    let delta_theta = cos_sep.clamp(-1.0, 1.0).acos();

    if delta_theta >= theta_s + theta_o {
        ShadowState::FullLight
    } else if delta_theta <= (theta_o - theta_s).max(0.0) {
        ShadowState::Umbra
    } else if delta_theta <= (theta_s - theta_o).max(0.0) {
        ShadowState::Antumbra
    } else {
        ShadowState::Penumbra
    }
}

pub fn conical_shadow_fraction(
    point: Position,
    light_source_position: Position,
    light_source_radius: Length,
    occluder_position: Position,
    occluder_radius: Length,
) -> f64 {
    let r_s = light_source_radius.value();
    let r_o = occluder_radius.value();

    if r_s <= 0.0 || r_o <= 0.0 || !r_s.is_finite() || !r_o.is_finite() {
        return 0.0;
    }

    let p = point.raw();
    let s = light_source_position.raw();
    let o = occluder_position.raw();

    let v_s = s - p;
    let v_o = o - p;

    let d_s = v_s.magnitude();
    let d_o = v_o.magnitude();

    if d_s <= 0.0 || d_o <= 0.0 || !d_s.is_finite() || !d_o.is_finite() || d_o >= d_s {
        return 0.0;
    }

    let u_s = v_s / d_s;
    let u_o = v_o / d_o;

    let cos_sep = u_s.dot(&u_o);
    if cos_sep <= 0.0 || !cos_sep.is_finite() {
        return 0.0;
    }

    let r1 = (r_s / d_s).clamp(0.0, 1.0).asin();
    let r2 = (r_o / d_o).clamp(0.0, 1.0).asin();
    let d = cos_sep.clamp(-1.0, 1.0).acos();

    if r1 <= 0.0 {
        return 0.0;
    }

    if d >= r1 + r2 {
        0.0
    } else if d <= (r2 - r1).max(0.0) {
        1.0
    } else if d <= (r1 - r2).max(0.0) {
        let frac = (r2 / r1).powi(2);
        frac.clamp(0.0, 1.0)
    } else {
        let d2 = d * d;
        let r1_2 = r1 * r1;
        let r2_2 = r2 * r2;

        let cos_alpha = ((d2 + r1_2 - r2_2) / (2.0 * d * r1)).clamp(-1.0, 1.0);
        let cos_beta = ((d2 + r2_2 - r1_2) / (2.0 * d * r2)).clamp(-1.0, 1.0);

        let alpha = cos_alpha.acos();
        let beta = cos_beta.acos();

        let tri_term = ((-d + r1 + r2) * (d + r1 - r2) * (d - r1 + r2) * (d + r1 + r2)).max(0.0);
        let area = r1_2 * alpha + r2_2 * beta - 0.5 * tri_term.sqrt();
        let source_area = PI * r1_2;

        (area / source_area).clamp(0.0, 1.0)
    }
}

pub fn is_in_conical_umbra(
    point: Position,
    light_source_position: Position,
    light_source_radius: Length,
    occluder_position: Position,
    occluder_radius: Length,
) -> bool {
    conical_shadow_state(
        point,
        light_source_position,
        light_source_radius,
        occluder_position,
        occluder_radius,
    ) == ShadowState::Umbra
}

pub fn is_in_conical_penumbra(
    point: Position,
    light_source_position: Position,
    light_source_radius: Length,
    occluder_position: Position,
    occluder_radius: Length,
) -> bool {
    conical_shadow_state(
        point,
        light_source_position,
        light_source_radius,
        occluder_position,
        occluder_radius,
    ) == ShadowState::Penumbra
}

pub fn is_in_conical_antumbra(
    point: Position,
    light_source_position: Position,
    light_source_radius: Length,
    occluder_position: Position,
    occluder_radius: Length,
) -> bool {
    conical_shadow_state(
        point,
        light_source_position,
        light_source_radius,
        occluder_position,
        occluder_radius,
    ) == ShadowState::Antumbra
}

pub fn conical_umbra_length(
    light_source_position: Position,
    light_source_radius: Length,
    occluder_position: Position,
    occluder_radius: Length,
) -> Length {
    let r_s = light_source_radius.value();
    let r_o = occluder_radius.value();
    let d_so = (occluder_position.raw() - light_source_position.raw()).magnitude();

    if r_s <= r_o || d_so <= 0.0 || !r_s.is_finite() || !r_o.is_finite() || !d_so.is_finite() {
        return Length::new(f64::INFINITY);
    }

    let l_u = d_so * (r_o / (r_s - r_o));
    Length::new(l_u)
}

pub fn conical_penumbra_length(
    light_source_position: Position,
    light_source_radius: Length,
    occluder_position: Position,
    occluder_radius: Length,
) -> Length {
    let r_s = light_source_radius.value();
    let r_o = occluder_radius.value();
    let d_so = (occluder_position.raw() - light_source_position.raw()).magnitude();

    if r_s <= 0.0
        || r_o <= 0.0
        || d_so <= 0.0
        || !r_s.is_finite()
        || !r_o.is_finite()
        || !d_so.is_finite()
    {
        return Length::new(0.0);
    }

    let l_p = d_so * (r_o / (r_s + r_o));
    Length::new(l_p)
}

pub fn multi_occluder_shadow_fraction(
    point: Position,
    light_source_position: Position,
    light_source_radius: Length,
    occluders: &[(Position, Length)],
) -> f64 {
    let mut total_frac: f64 = 0.0;
    for &(occ_pos, occ_radius) in occluders {
        let frac = conical_shadow_fraction(
            point,
            light_source_position,
            light_source_radius,
            occ_pos,
            occ_radius,
        );
        total_frac = total_frac.max(frac);
        if total_frac >= 1.0 {
            return 1.0;
        }
    }
    total_frac
}

pub fn multi_occluder_shadow_state(
    point: Position,
    light_source_position: Position,
    light_source_radius: Length,
    occluders: &[(Position, Length)],
) -> ShadowState {
    let mut has_antumbra = false;
    let mut has_penumbra = false;

    for &(occ_pos, occ_radius) in occluders {
        let state = conical_shadow_state(
            point,
            light_source_position,
            light_source_radius,
            occ_pos,
            occ_radius,
        );
        match state {
            ShadowState::Umbra => return ShadowState::Umbra,
            ShadowState::Antumbra => has_antumbra = true,
            ShadowState::Penumbra => has_penumbra = true,
            ShadowState::FullLight => {}
        }
    }

    if has_penumbra {
        ShadowState::Penumbra
    } else if has_antumbra {
        ShadowState::Antumbra
    } else {
        ShadowState::FullLight
    }
}

pub fn illuminance_factor(
    point: Position,
    light_source_position: Position,
    light_source_radius: Length,
    occluders: &[(Position, Length)],
) -> f64 {
    let shadow_frac = multi_occluder_shadow_fraction(
        point,
        light_source_position,
        light_source_radius,
        occluders,
    );
    (1.0 - shadow_frac).clamp(0.0, 1.0)
}