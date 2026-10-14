mod engine;
mod forecast_reader;
mod orbit_preview;
mod snapshot;
mod vehicle;

pub use engine::Engine;
pub use forecast_reader::{ForecastControl, PreviewEngine};

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
        reference_body_id: String,
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
        has_local_up_body: bool,
        local_up_body: FfiVec3,
        main_engine_loads: Vec<MainEngineLoad>,
        has_battery: bool,
        battery_stored_j: f64,
        battery_capacity_j: f64,
        fuel_reserves: Vec<FuelReserve>,
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

    #[derive(Debug, PartialEq)]
    struct MainEngineLoad {
        instance_id: String,
        load_fraction: f64,
    }

    #[derive(Debug, PartialEq)]
    struct FuelReserve {
        name: String,
        stored_kg: f64,
        capacity_kg: f64,
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
        has_min_throttle: bool,
        min_throttle_fraction: f64,
    }

    #[derive(Debug)]
    struct FlightEvent {
        kind: u8,
        total_epoch_seconds: f64,
    }

    #[derive(Debug, PartialEq)]
    struct FfiForecastArc {
        reference_body_id: String,
        start_seconds: f64,
        end_seconds: f64,
        points_m: Vec<FfiVec3>,
    }

    #[derive(Debug, PartialEq)]
    struct FfiForecastEvent {
        kind: u8,
        body_name: String,
        seconds: f64,
        position_m: FfiVec3,
    }

    #[derive(Debug, PartialEq)]
    struct FfiForecastBody {
        name: String,
        radius_m: f64,
        position_m: FfiVec3,
    }

    #[derive(Debug, PartialEq)]
    struct OrbitPreview {
        reference_body_id: String,
        reference_body_name: String,
        orbit_classification: u8,
        stellar_reference: bool,
        source_epoch_seconds: f64,
        horizon_seconds: f64,
        rendered_horizon_seconds: f64,
        end_reason: u8,
        has_impact: bool,
        impact_epoch_seconds: f64,
        has_soi: bool,
        soi_radius_m: f64,
        soi_outside: bool,
        has_soi_exit: bool,
        soi_exit_seconds: f64,
        soi_exit_position_m: FfiVec3,
        under_thrust: bool,
        has_orbit_view_basis: bool,
        has_inclination: bool,
        inclination_rad: f64,
        has_ascending_longitude: bool,
        ascending_longitude_rad: f64,
        has_next_ascending: bool,
        next_ascending_seconds: f64,
        next_ascending_position_m: FfiVec3,
        has_next_descending: bool,
        next_descending_seconds: f64,
        next_descending_position_m: FfiVec3,
        orbit_horizontal: FfiVec3,
        orbit_vertical: FfiVec3,
        has_equatorial_reference: bool,
        equatorial_normal: FfiVec3,
        equatorial_horizontal: FfiVec3,
        equatorial_vertical: FfiVec3,
        equatorial_has_inclination: bool,
        equatorial_inclination_rad: f64,
        equatorial_has_ascending_longitude: bool,
        equatorial_ascending_longitude_rad: f64,
        equatorial_has_next_ascending: bool,
        equatorial_next_ascending_seconds: f64,
        equatorial_next_ascending_position_m: FfiVec3,
        equatorial_has_next_descending: bool,
        equatorial_next_descending_seconds: f64,
        equatorial_next_descending_position_m: FfiVec3,
        has_eccentricity: bool,
        eccentricity: f64,
        has_periapsis: bool,
        periapsis_altitude_m: f64,
        has_apoapsis: bool,
        apoapsis_altitude_m: f64,
        has_period: bool,
        period_seconds: f64,
        has_next_periapsis: bool,
        next_periapsis_seconds: f64,
        next_periapsis_position_m: FfiVec3,
        has_next_apoapsis: bool,
        next_apoapsis_seconds: f64,
        next_apoapsis_position_m: FfiVec3,
        relative_points_m: Vec<FfiVec3>,
        osculating_closed_points_m: Vec<FfiVec3>,
        future_arcs: Vec<FfiForecastArc>,
        forecast_events: Vec<FfiForecastEvent>,
        forecast_bodies: Vec<FfiForecastBody>,
        forecast_target_seconds: f64,
        forecast_completed_seconds: f64,
        forecast_complete: bool,
    }

    unsafe extern "C++" {
        include!("rocketcon_forecast_progress.hpp");
        type ForecastProgress;
        fn publish(self: Pin<&mut ForecastProgress>, preview: OrbitPreview, generation: u64);
    }

    extern "Rust" {
        type Engine;

        type PreviewEngine;
        type ForecastControl;

        fn create_forecast_control() -> Box<ForecastControl>;
        fn select_generation(self: &ForecastControl, generation: u64);
        fn create_preview_engine() -> Result<Box<PreviewEngine>>;
        fn load_read_only(
            self: &mut PreviewEngine,
            save_path_utf8: &str,
            vehicle_uuid: &str,
        ) -> Result<()>;
        fn predict(
            self: &PreviewEngine,
            under_thrust: bool,
            control: &ForecastControl,
            generation: u64,
            progress: Pin<&mut ForecastProgress>,
        ) -> Result<OrbitPreview>;

        fn create_engine() -> Result<Box<Engine>>;
        fn load_save(self: &mut Engine, save_path_utf8: &str, vehicle_uuid: &str) -> Result<()>;
        fn set_control(self: &mut Engine, pitch: f64, yaw: f64, roll: f64) -> Result<()>;
        fn set_main_engine_load(
            self: &mut Engine,
            instance_id: &str,
            load_fraction: f64,
        ) -> Result<()>;
        fn set_main_engine_ramp(self: &mut Engine, instance_id: &str, direction: i8) -> Result<()>;
        fn step(self: &mut Engine, dt_seconds: f64) -> Result<()>;
        fn snapshot(self: &Engine) -> Result<FlightSnapshot>;
        fn vehicle_components(self: &Engine) -> Result<Vec<VisualComponent>>;
        fn orbit_preview(self: &Engine) -> Result<OrbitPreview>;
        fn take_events(self: &mut Engine) -> Result<Vec<FlightEvent>>;
        fn save(self: &Engine) -> Result<()>;
    }
}

pub use engine::create_engine;
pub use forecast_reader::{create_forecast_control, create_preview_engine};
