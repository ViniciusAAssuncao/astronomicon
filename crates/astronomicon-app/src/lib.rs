pub mod ephemeris;
pub mod error;

pub use ephemeris::*;
pub use error::{AppError, AppResult};

pub fn run() -> AppResult<()> {
    println!("Astronomicon iniciado.");
    Ok(())
}