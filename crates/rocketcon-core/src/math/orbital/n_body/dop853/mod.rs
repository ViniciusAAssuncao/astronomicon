mod coefficients;
mod dense;
mod gravity;
mod solver;

pub use dense::Dop853DenseSegment;
pub use gravity::{
    GravitySource, inertial_point_mass_derivative, inertial_point_mass_derivative_from_sources,
};
pub use solver::{Dop853Config, Dop853Error, Dop853Result, solve_dop853, solve_dop853_with_event};

#[cfg(test)]
mod tests;
