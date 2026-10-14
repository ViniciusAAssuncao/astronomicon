mod coefficients;
mod dense;
mod gravity;
mod solver;

pub use dense::Dop853DenseSegment;
pub use gravity::{GravitySource, inertial_point_mass_derivative};
pub use solver::{Dop853Config, Dop853Error, Dop853Result, solve_dop853};

#[cfg(test)]
mod tests;
