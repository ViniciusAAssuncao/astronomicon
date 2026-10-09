mod engine;
mod snapshot;
mod vehicle;

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
        has_reference_body: bool,
        reference_body_position_m: FfiVec3,
        reference_body_radius_m: f64,
        velocity_m_s: FfiVec3,
        angular_velocity_rad_s: FfiVec3,
        speed_m_s: f64,
        has_reference_speed: bool,
        reference_speed_m_s: f64,
        has_reference_vertical_speed: bool,
        reference_vertical_speed_m_s: f64,
        has_reference_horizontal_speed: bool,
        reference_horizontal_speed_m_s: f64,
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

    #[derive(Debug)]
    struct VisualComponent {
        instance_id: String,
        name: String,
        kind: u8,
        stage_index: u32,
        offset_x_m: f64,
        offset_y_m: f64,
        offset_z_m: f64,
        length_m: f64,
        diameter_m: f64,
    }

    #[derive(Debug)]
    struct FlightEvent {
        kind: u8,
        total_epoch_seconds: f64,
    }

    extern "Rust" {
        type Engine;

        fn create_engine() -> Result<Box<Engine>>;
        fn load_save(self: &mut Engine, save_path_utf8: &str, vehicle_uuid: &str) -> Result<()>;
        fn set_control(self: &mut Engine, pitch: f64, yaw: f64, roll: f64) -> Result<()>;
        fn step(self: &mut Engine, dt_seconds: f64) -> Result<()>;
        fn snapshot(self: &Engine) -> Result<FlightSnapshot>;
        fn vehicle_components(self: &Engine) -> Result<Vec<VisualComponent>>;
        fn take_events(self: &mut Engine) -> Result<Vec<FlightEvent>>;
        fn save(self: &Engine) -> Result<()>;
    }
}

pub use engine::create_engine;
