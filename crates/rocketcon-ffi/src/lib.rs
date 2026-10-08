mod engine;
mod snapshot;

pub use engine::Engine;

#[cxx::bridge(namespace = "rocketcon")]
pub mod ffi {
    #[derive(Debug, PartialEq)]
    struct FfiVec3 {
        x: f64,
        y: f64,
        z: f64,
    }

    #[derive(Debug, PartialEq)]
    struct FlightSnapshot {
        total_epoch_seconds: f64,
        position_m: FfiVec3,
        velocity_m_s: FfiVec3,
        speed_m_s: f64,
        has_altitude: bool,
        altitude_m: f64,
        has_mach: bool,
        mach: f64,
        has_dynamic_pressure: bool,
        dynamic_pressure_pa: f64,
        has_g_load: bool,
        total_g_load: f64,
        has_surface_contact: bool,
        surface_contact: bool,
    }

    extern "Rust" {
        type Engine;

        fn create_engine() -> Result<Box<Engine>>;
        fn load_save(self: &mut Engine, save_path_utf8: &str, vehicle_uuid: &str) -> Result<()>;
        fn set_control(self: &mut Engine, pitch: f64, yaw: f64, roll: f64) -> Result<()>;
        fn step(self: &mut Engine, dt_seconds: f64) -> Result<()>;
        fn snapshot(self: &Engine) -> Result<FlightSnapshot>;
        fn save(self: &Engine) -> Result<()>;
    }
}

pub use engine::create_engine;
