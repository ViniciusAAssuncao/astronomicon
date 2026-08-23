pub mod black_hole;
pub mod climate;
pub mod context;
pub mod ephemeris;
pub mod error;
pub mod geology;
pub mod geophysics;
pub mod gravity;
pub mod hydrosphere;
pub mod lagrange;
pub mod mineralogy;
pub mod minor_planet;
pub mod radiation;
pub mod resonance;
pub mod seismology;
pub mod shape;
pub mod sky;
pub mod tidal;
pub mod volcanism;

pub use black_hole::*;
pub use climate::*;
pub use context::{build_context, AppContext};
pub use ephemeris::*;
pub use error::{AppError, AppResult};
pub use geology::*;
pub use geophysics::*;
pub use gravity::*;
pub use hydrosphere::*;
pub use lagrange::*;
pub use mineralogy::*;
pub use minor_planet::*;
pub use radiation::*;
pub use resonance::*;
pub use seismology::*;
pub use shape::*;
pub use sky::*;
pub use tidal::*;
pub use volcanism::*;

pub fn run() -> AppResult<()> {
    let rt = tokio::runtime::Runtime::new()?;
    let _ctx = rt.block_on(build_context())?;
    println!("Astronomicon iniciado.");
    Ok(())
}