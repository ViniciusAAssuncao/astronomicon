use super::dd::{delta_minus_sin, one_minus_cos, Dd};

type Triplet = [Dd; 3];

fn dot(a: Triplet, b: Triplet) -> Dd {
    a[0].mul(b[0]).add(a[1].mul(b[1])).add(a[2].mul(b[2]))
}

fn input(x: [f64; 3]) -> Triplet {
    [Dd::from(x[0]), Dd::from(x[1]), Dd::from(x[2])]
}

fn parameters(
    position: [f64; 3],
    velocity: [f64; 3],
    mu: f64,
) -> Option<(Triplet, Triplet, Dd, Dd, Dd, Dd)> {
    if !(mu > 0.0 && mu.is_finite())
        || !position
            .iter()
            .chain(velocity.iter())
            .all(|v| v.is_finite())
    {
        return None;
    }
    let r = input(position);
    let v = input(velocity);
    let r2 = dot(r, r);
    if !(r2.hi > 0.0 && r2.hi.is_finite()) {
        return None;
    }
    let r0 = r2.sqrt();
    let gm = Dd::from(mu);

    let rv = dot(r, v);
    let v2 = dot(v, v);
    let alpha = Dd::from(2.0).div(r0).sub(v2.div(gm));
    if !(alpha.hi > 0.0 && alpha.hi.is_finite()) {
        return None;
    }
    let h = [
        r[1].mul(v[2]).sub(r[2].mul(v[1])),
        r[2].mul(v[0]).sub(r[0].mul(v[2])),
        r[0].mul(v[1]).sub(r[1].mul(v[0])),
    ];
    let h2 = dot(h, h);
    if !(h2.hi > 0.0 && h2.hi.is_finite()) {
        return None;
    }
    let root_alpha = alpha.sqrt();
    let eccentric_cos = Dd::from(1.0).sub(alpha.mul(r0));
    let eccentric_sin = rv.mul(root_alpha).div(gm.sqrt());
    if ![root_alpha.hi, eccentric_cos.hi, eccentric_sin.hi]
        .iter()
        .all(|x| x.is_finite())
    {
        return None;
    }
    Some((r, v, r0, alpha, eccentric_cos, eccentric_sin))
}

pub(super) fn elliptic_anomaly_dd(
    alpha: Dd,
    root_alpha: Dd,
    eccentric_cos: Dd,
    eccentric_sin: Dd,
    mu: f64,
    dt: f64,
) -> Option<(Dd, usize)> {
    let zero = Dd::from(0.0);
    let one = Dd::from(1.0);
    let pi = Dd {
        hi: std::f64::consts::PI,
        lo: 1.224_646_799_147_353_2e-16,
    };
    let mean = alpha
        .mul(root_alpha)
        .mul(Dd::from(mu).sqrt())
        .mul(Dd::from(dt));
    let target = pi.add(eccentric_sin.mul(Dd::from(2.0))).sub(mean);
    let evaluate = |delta: Dd| {
        one.add(eccentric_cos)
            .mul(delta)
            .sub(eccentric_cos.mul(delta_minus_sin(delta)))
            .add(eccentric_sin.mul(one_minus_cos(delta)))
    };
    let mut lo = Dd::from(-0.25);
    let mut hi = Dd::from(0.25);
    if evaluate(lo).above(target) || target.above(evaluate(hi)) {
        return None;
    }
    let mut delta = zero;
    for iteration in 0..144 {
        let residual = evaluate(delta).sub(target);
        if residual.above(zero) {
            hi = delta;
        } else {
            lo = delta;
        }
        let chi_low = pi.sub(hi).div(root_alpha).to_f64();
        let chi_high = pi.sub(lo).div(root_alpha).to_f64();
        if chi_low.is_finite() && chi_low == chi_high {
            return Some((pi.sub(hi).div(root_alpha), iteration + 1));
        }
        let sine = delta.sub(delta_minus_sin(delta));
        let slope = one
            .add(eccentric_cos)
            .sub(eccentric_cos.mul(one_minus_cos(delta)))
            .add(eccentric_sin.mul(sine));
        if !(slope.hi > 0.0 && slope.hi.is_finite()) {
            return None;
        }
        let candidate = delta.sub(residual.div(slope));
        let next = if candidate.above(lo) && hi.above(candidate) {
            candidate
        } else {
            lo.add(hi.sub(lo).mul(Dd::from(0.5)))
        };
        if next.hi == delta.hi && next.lo == delta.lo {
            let root = pi.sub(delta).div(root_alpha);
            return root.to_f64().is_finite().then_some((root, iteration + 1));
        }
        delta = next;
    }
    None
}

fn oriented_elliptic_trial(
    position: [f64; 3],
    velocity: [f64; 3],
    mu: f64,
    dt: f64,
) -> Option<([f64; 3], [f64; 3], f64)> {
    if !(dt > 0.0 && dt.is_finite()) {
        return None;
    }
    let (_, _, _, alpha, ecos, esin) = parameters(position, velocity, mu)?;
    let root_alpha = alpha.sqrt();
    let (chi, _) = elliptic_anomaly_dd(alpha, root_alpha, ecos, esin, mu, dt)?;
    let (p, v) = oriented_reconstruction(position, velocity, mu, dt, chi)?;
    Some((p, v, chi.to_f64()))
}

fn oriented_reconstruction(
    position: [f64; 3],
    velocity: [f64; 3],
    mu: f64,
    dt: f64,
    chi: Dd,
) -> Option<([f64; 3], [f64; 3])> {
    let (r, v, r0, alpha, ecos, esin) = parameters(position, velocity, mu)?;
    let root_alpha = alpha.sqrt();
    let one = Dd::from(1.0);
    let pi = Dd {
        hi: std::f64::consts::PI,
        lo: 1.224_646_799_147_353_2e-16,
    };

    let delta = pi.sub(root_alpha.mul(chi));
    if !(delta.hi.is_finite() && delta.hi.abs() <= 0.25) {
        return None;
    }
    let sin = delta.sub(delta_minus_sin(delta));
    let small_cos = one_minus_cos(delta);
    let q = Dd::from(2.0).sub(small_cos).div(alpha);
    let f = one.sub(q.div(r0));
    let e_minus_sin = pi.sub(delta).sub(sin);
    let g = Dd::from(dt).sub(e_minus_sin.div(alpha.mul(root_alpha).mul(Dd::from(mu).sqrt())));
    let radius_ratio = one.add(ecos).sub(ecos.mul(small_cos)).add(esin.mul(sin));
    let radius = radius_ratio.div(alpha);
    if !(radius.hi > 0.0 && radius.hi.is_finite()) {
        return None;
    }
    let fdot = Dd::from(-1.0)
        .mul(Dd::from(mu).sqrt())
        .mul(sin)
        .div(root_alpha.mul(radius).mul(r0));
    let gdot = one.sub(q.div(radius));
    let p = std::array::from_fn(|i| r[i].mul(f).add(v[i].mul(g)).to_f64());
    let w = std::array::from_fn(|i| r[i].mul(fdot).add(v[i].mul(gdot)).to_f64());
    p.iter()
        .chain(w.iter())
        .all(|x| x.is_finite())
        .then_some((p, w))
}

pub(super) fn try_propagate(
    position: [f64; 3],
    velocity: [f64; 3],
    mu: f64,
    dt: f64,
) -> Option<([f64; 3], [f64; 3], f64)> {
    if dt < 0.0 && dt.is_finite() {
        let opposite = [-velocity[0], -velocity[1], -velocity[2]];
        return try_propagate(position, opposite, mu, -dt)
            .map(|(p, v, chi)| (p, [-v[0], -v[1], -v[2]], -chi));
    }
    if !(mu > 0.0 && dt > 0.0 && mu.is_finite() && dt.is_finite()) {
        return None;
    }
    if !position
        .iter()
        .chain(velocity.iter())
        .all(|x| x.is_finite())
    {
        return None;
    }
    let radius = position[0].hypot(position[1]).hypot(position[2]);
    if !(radius > 0.0 && radius.is_finite()) {
        return None;
    }
    let radial_product = position[0].mul_add(
        velocity[0],
        position[1].mul_add(velocity[1], position[2] * velocity[2]),
    );
    let speed2 = velocity[0].mul_add(
        velocity[0],
        velocity[1].mul_add(velocity[1], velocity[2] * velocity[2]),
    );
    let radial_speed = radial_product / radius;
    let tangential2 = (speed2 - radial_speed * radial_speed).max(0.0);
    let transverse_ratio = (radius / mu) * tangential2;
    if !(transverse_ratio.is_finite() && transverse_ratio <= 1.0e-4) {
        return None;
    }
    let alpha = 2.0 / radius - speed2 / mu;
    if !(alpha > 0.0 && alpha.is_finite()) {
        return None;
    }
    let root_alpha = alpha.sqrt();
    let mean = ((mu.sqrt() * dt) * alpha) * root_alpha;
    let eccentric_sin = (radial_product / mu.sqrt()) * root_alpha;
    if !(mean.is_finite() && eccentric_sin.is_finite()) {
        return None;
    }
    if (mean - (std::f64::consts::PI + 2.0 * eccentric_sin)).abs() > 0.01 {
        return None;
    }
    oriented_elliptic_trial(position, velocity, mu, dt)
}
