#[derive(Clone, Copy)]
pub(super) struct Dd {
    pub(super) hi: f64,
    pub(super) lo: f64,
}

impl Dd {
    pub(super) fn from(value: f64) -> Self {
        Self { hi: value, lo: 0.0 }
    }

    pub(super) fn normalized(hi: f64, lo: f64) -> Self {
        let sum = hi + lo;
        Self {
            hi: sum,
            lo: lo - (sum - hi),
        }
    }

    pub(super) fn add(self, other: Self) -> Self {
        let sum = self.hi + other.hi;
        let bb = sum - self.hi;
        let err = (self.hi - (sum - bb)) + (other.hi - bb);
        Self::normalized(sum, err + self.lo + other.lo)
    }

    pub(super) fn sub(self, other: Self) -> Self {
        self.add(Self {
            hi: -other.hi,
            lo: -other.lo,
        })
    }

    pub(super) fn mul(self, other: Self) -> Self {
        let product = self.hi * other.hi;
        let err = self.hi.mul_add(other.hi, -product)
            + self.hi * other.lo
            + self.lo * other.hi
            + self.lo * other.lo;
        Self::normalized(product, err)
    }

    pub(super) fn div(self, other: Self) -> Self {
        let first = Self::from(self.hi / other.hi);
        let rem = self.sub(other.mul(first));
        let second = Self::from(rem.hi / other.hi);
        let answer = first.add(second);
        let rem = self.sub(other.mul(answer));
        answer.add(Self::from(rem.hi / other.hi))
    }

    pub(super) fn sqrt(self) -> Self {
        let first = Self::from(self.hi.sqrt());
        let correction = self.sub(first.mul(first)).div(Self::from(2.0 * first.hi));
        let answer = first.add(correction);
        let correction = self
            .sub(answer.mul(answer))
            .div(Self::from(2.0 * answer.hi));
        answer.add(correction)
    }

    pub(super) fn above(self, other: Self) -> bool {
        self.hi > other.hi || (self.hi == other.hi && self.lo > other.lo)
    }

    pub(super) fn to_f64(self) -> f64 {
        self.hi + self.lo
    }
}

pub(super) fn delta_minus_sin(delta: Dd) -> Dd {
    let squared = delta.mul(delta);
    let mut term = delta.mul(squared).div(Dd::from(6.0));
    let mut result = term;
    for n in 1..10 {
        term = term
            .mul(squared)
            .div(Dd::from(-((2 * n + 2) * (2 * n + 3)) as f64));
        result = result.add(term);
    }
    result
}

pub(super) fn one_minus_cos(delta: Dd) -> Dd {
    let squared = delta.mul(delta);
    let mut term = squared.div(Dd::from(2.0));
    let mut result = term;
    for n in 1..10 {
        term = term
            .mul(squared)
            .div(Dd::from(-((2 * n + 1) * (2 * n + 2)) as f64));
        result = result.add(term);
    }
    result
}
