pub mod climate;
pub mod context;
pub mod ephemeris;
pub mod error;
pub mod gravity;

pub use climate::*;
pub use context::{build_context, AppContext};
pub use ephemeris::*;
pub use error::{AppError, AppResult};
pub use gravity::*;

pub fn run() -> AppResult<()> {
    let rt = tokio::runtime::Runtime::new()?;
    let _ctx = rt.block_on(build_context())?;
    println!("Astronomicon iniciado.");
    Ok(())
}