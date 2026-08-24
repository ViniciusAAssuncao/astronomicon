use crate::units::{Length, Vector3};

pub fn ray_sphere_intersections(
    ray_origin: Vector3,
    ray_dir: Vector3,
    sphere_radius: Length,
) -> Option<(Length, Length)> {
    let r = sphere_radius.value();
    let d = ray_dir.normalized();

    if r <= 0.0 || !r.is_finite() {
        return None;
    }

    let b = ray_origin.dot(&d);
    let c = ray_origin.dot(&ray_origin) - r * r;
    let disc = b * b - c;

    if disc < 0.0 {
        return None;
    }

    let sqrt_disc = disc.sqrt();
    let t1 = -b - sqrt_disc;
    let t2 = -b + sqrt_disc;

    Some((Length::new(t1), Length::new(t2)))
}

pub fn ray_atmosphere_segment(
    ray_origin: Vector3,
    ray_dir: Vector3,
    planet_radius: Length,
    atmosphere_top_radius: Length,
) -> Option<(Length, Length, bool)> {
    let r_p = planet_radius.value();
    let r_atm = atmosphere_top_radius.value();
    let d = ray_dir.normalized();

    if r_p <= 0.0 || r_atm <= r_p || !r_p.is_finite() || !r_atm.is_finite() {
        return None;
    }

    let r0_sq = ray_origin.dot(&ray_origin);
    let r0 = r0_sq.sqrt();

    let b_atm = ray_origin.dot(&d);
    let c_atm = r0_sq - r_atm * r_atm;
    let disc_atm = b_atm * b_atm - c_atm;

    if disc_atm < 0.0 {
        return None;
    }

    let sqrt_disc_atm = disc_atm.sqrt();
    let t_atm1 = -b_atm - sqrt_disc_atm;
    let t_atm2 = -b_atm + sqrt_disc_atm;

    if t_atm2 <= 0.0 {
        return None;
    }

    let t_start = t_atm1.max(0.0);
    let mut t_end = t_atm2;
    let mut hits_ground = false;

    let b_p = ray_origin.dot(&d);
    let c_p = r0_sq - r_p * r_p;
    let disc_p = b_p * b_p - c_p;

    if disc_p >= 0.0 {
        let sqrt_disc_p = disc_p.sqrt();
        let t_p1 = -b_p - sqrt_disc_p;
        let t_p2 = -b_p + sqrt_disc_p;

        if t_p1 > 1e-6 {
            if t_p1 < t_end {
                t_end = t_p1;
                hits_ground = true;
            }
        } else if t_p2 > 1e-6 && b_p < 0.0 {
            if r0 <= r_p + 1.0 {
                return None;
            }
            t_end = 0.0;
            hits_ground = true;
        }
    }

    if t_end <= t_start + 1e-6 {
        return None;
    }

    Some((Length::new(t_start), Length::new(t_end), hits_ground))
}
