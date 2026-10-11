pub fn brent_root_find<F>(
    mut f: F,
    mut a: f64,
    mut b: f64,
    tol: f64,
    max_iter: usize,
) -> Result<f64, ()>
where
    F: FnMut(f64) -> f64,
{
    let mut fa = f(a);
    let mut fb = f(b);

    if fa * fb > 0.0 {
        return Err(());
    }

    if fa.abs() < fb.abs() {
        std::mem::swap(&mut a, &mut b);
        std::mem::swap(&mut fa, &mut fb);
    }

    let mut c = a;
    let mut fc = fa;
    let mut mflag = true;
    let mut d = 0.0;

    for _ in 0..max_iter {
        if fb.abs() <= tol || (b - a).abs() <= tol {
            return Ok(b);
        }

        let s = if (fa - fc).abs() > 1e-15 && (fb - fc).abs() > 1e-15 {
            (a * fb * fc) / ((fa - fb) * (fa - fc))
                + (b * fa * fc) / ((fb - fa) * (fb - fc))
                + (c * fa * fb) / ((fc - fa) * (fc - fb))
        } else {
            b - (fb * (b - a)) / (fb - fa)
        };

        let cond1 = (s < (3.0 * a + b) * 0.25 && s < b) || (s > (3.0 * a + b) * 0.25 && s > b);
        let cond2 = mflag && (s - b).abs() >= (b - c).abs() * 0.5;
        let cond3 = !mflag && (s - b).abs() >= (c - d).abs() * 0.5;
        let cond4 = mflag && (b - c).abs() < tol;
        let cond5 = !mflag && (c - d).abs() < tol;

        let s_final = if cond1 || cond2 || cond3 || cond4 || cond5 {
            mflag = true;
            0.5 * (a + b)
        } else {
            mflag = false;
            s
        };

        let fs = f(s_final);
        d = c;
        c = b;
        fc = fb;

        if fa * fs < 0.0 {
            b = s_final;
            fb = fs;
        } else {
            a = s_final;
            fa = fs;
        }

        if fa.abs() < fb.abs() {
            std::mem::swap(&mut a, &mut b);
            std::mem::swap(&mut fa, &mut fb);
        }
    }

    Ok(b)
}