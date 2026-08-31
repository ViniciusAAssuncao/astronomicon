pub mod manager;
pub mod migrations;
pub mod paths;
pub mod template;

pub use manager::resolve_current_save_pool;
pub use paths::SAVES_DIRECTORY;