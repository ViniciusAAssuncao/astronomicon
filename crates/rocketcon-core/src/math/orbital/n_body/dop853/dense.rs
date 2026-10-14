#[derive(Debug, Clone, PartialEq)]
pub struct Dop853DenseSegment<const N: usize> {
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub valid_until_seconds: f64,
    pub start_state: [f64; N],
    pub coefficients: [[f64; N]; 7],
}

impl<const N: usize> Dop853DenseSegment<N> {
    pub fn position_segment(&self) -> Option<Dop853DenseSegment<3>> {
        if N < 3 {
            return None;
        }
        let mut coefficients = [[0.0; 3]; 7];
        for (row, source) in coefficients.iter_mut().zip(&self.coefficients) {
            row.copy_from_slice(&source[..3]);
        }
        Some(Dop853DenseSegment {
            start_seconds: self.start_seconds,
            end_seconds: self.end_seconds,
            valid_until_seconds: self.valid_until_seconds,
            start_state: [
                self.start_state[0],
                self.start_state[1],
                self.start_state[2],
            ],
            coefficients,
        })
    }

    pub fn evaluate(&self, seconds: f64) -> Option<[f64; N]> {
        if !seconds.is_finite()
            || self.end_seconds <= self.start_seconds
            || seconds < self.start_seconds
            || seconds > self.valid_until_seconds
        {
            return None;
        }
        let x = (seconds - self.start_seconds) / (self.end_seconds - self.start_seconds);
        let mut polynomial = [0.0; N];
        for (index, coefficients) in self.coefficients.iter().rev().enumerate() {
            let factor = if index % 2 == 0 { x } else { 1.0 - x };
            for component in 0..N {
                polynomial[component] = (polynomial[component] + coefficients[component]) * factor;
            }
        }
        for (value, origin) in polynomial.iter_mut().zip(self.start_state) {
            *value += origin;
        }
        Some(polynomial)
    }
}
