pub mod error;

pub use error::{AppError, AppResult};

pub fn run() -> AppResult<()> {
    println!("Astronomicon iniciado.");
    Ok(())
}