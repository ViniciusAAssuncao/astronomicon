use astronomicon_core::units::{
    Duration, GravitationalParameter, Position, Vector3, VelocityVector,
};
use rocketcon_core::math::orbital::universal::propagate_universal_state_vectors;

fn propagate(r: Vector3, v: Vector3, mu: f64, dt: f64) -> (Vector3, Vector3) {
    let (p, vel) = propagate_universal_state_vectors(
        Position::from_raw(r),
        VelocityVector::from_raw(v),
        GravitationalParameter::new(mu),
        Duration::new(dt),
    )
    .expect("valid non-collision state should propagate");
    (p.raw(), vel.raw())
}

fn next_unit(seed: &mut u64) -> f64 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    ((*seed >> 11) as f64) / ((1u64 << 53) as f64)
}

#[test]
fn adversarial_two_body_forward_reverse_and_invariants() {
    let mut seed = 0xC11_2026_10_12u64;
    let mut worst_energy = 0.0f64;
    let mut worst_position = 0.0f64;
    let mut worst_velocity = 0.0f64;
    let mut tested = 0usize;
    for _ in 0..1800 {
        let radius = 10.0f64.powf(4.0 + 8.0 * next_unit(&mut seed));
        let mu = 10.0f64.powf(11.0 + 11.0 * next_unit(&mut seed));
        let vc = (mu / radius).sqrt();
        let radial = (-0.8 + 1.6 * next_unit(&mut seed)) * vc;
        let transverse = (0.4 + 2.3 * next_unit(&mut seed)) * vc;
        let r = Vector3::new(radius, 0.0, 0.0);
        let v = Vector3::new(radial, transverse, 0.04 * vc);
        let timescale = (radius / vc) * 10.0f64.powf(-2.0 + 3.8 * next_unit(&mut seed));
        let dt = if next_unit(&mut seed) < 0.5 {
            -timescale
        } else {
            timescale
        };
        let (p, w) = propagate(r, v, mu, dt);
        let (rp, rv) = propagate(p, w, mu, -dt);
        let energy0 = 0.5 * v.dot(&v) - mu / radius;
        let energy1 = 0.5 * w.dot(&w) - mu / p.magnitude();
        let scale = 0.5 * v.dot(&v) + mu / radius;
        let ediff = (energy1 - energy0).abs() / scale;
        let pdiff = (rp - r).magnitude() / radius;
        let vdiff = (rv - v).magnitude() / vc;
        worst_energy = worst_energy.max(ediff);
        worst_position = worst_position.max(pdiff);
        worst_velocity = worst_velocity.max(vdiff);
        assert!(
            ediff < 5e-8,
            "energy case {tested} r={radius} mu={mu} v={v:?} dt={dt} error={ediff}"
        );
        assert!(
            pdiff < 5e-7 && vdiff < 5e-7,
            "reverse case {tested} r={radius} mu={mu} v={v:?} dt={dt} p={pdiff} v={vdiff}"
        );
        tested += 1;
    }
    eprintln!(
        "ADVERSARIAL tested={tested} worst_energy={worst_energy:.3e} worst_return_position={worst_position:.3e} worst_return_velocity={worst_velocity:.3e}"
    );
}

#[test]
fn near_parabolic_sign_and_time_scale_stress() {
    let mu: f64 = 3.986_004_418e14;
    let r = Vector3::new(7e6, 0.0, 0.0);
    let tau = (7e6_f64.powi(3) / mu).sqrt();
    let escape = (2.0 * mu / 7e6).sqrt();
    let mut count = 0;
    let mut worst = 0.0f64;
    for radial_factor in [-0.3, 0.0, 0.3] {
        for energy_offset in [-1e-4, -1e-8, -1e-12, 0.0, 1e-12, 1e-8, 1e-4] {
            let v = Vector3::new(
                radial_factor * escape,
                escape * (1.0 + energy_offset - radial_factor * radial_factor / 2.0),
                0.0,
            );
            for time_multiplier in [-1e7, -1e3, -1e-2, 1e-2, 1e3, 1e7] {
                let dt = time_multiplier * tau;
                let (p, w) = propagate(r, v, mu, dt);
                let e0 = 0.5 * v.dot(&v) - mu / r.magnitude();
                let e1 = 0.5 * w.dot(&w) - mu / p.magnitude();
                let scale = 0.5 * v.dot(&v) + mu / r.magnitude();
                let eerr = (e1 - e0).abs() / scale;
                worst = worst.max(eerr);
                assert!(
                    eerr < 5e-8,
                    "offset={energy_offset} radial={radial_factor} time={time_multiplier} eerr={eerr}"
                );
                count += 1;
            }
        }
    }
    eprintln!("PARABOLIC_SWEEP cases={count} worst_energy={worst:.3e}");
    assert_eq!(count, 126);
}

#[test]
fn zero_time_still_rejects_invalid_physical_state() {
    use rocketcon_core::math::orbital::universal::propagate_universal_state_vectors;
    let finite_r = Position::from_raw(Vector3::new(7e6, 0.0, 0.0));
    let velocity = VelocityVector::from_raw(Vector3::new(0.0, 7500.0, 0.0));
    let dt = Duration::new(0.0);
    assert!(propagate_universal_state_vectors(
        finite_r,
        velocity,
        GravitationalParameter::new(f64::NAN),
        dt
    )
    .is_err());
    assert!(propagate_universal_state_vectors(
        Position::from_raw(Vector3::new(f64::NAN, 0.0, 0.0)),
        velocity,
        GravitationalParameter::new(3.986e14),
        dt
    )
    .is_err());
}
