pub mod definition;
pub mod error;
pub mod hierarchy;
pub mod moon_phase;
pub mod season_tracking;
pub mod skeleton;
pub mod tick;

pub use definition::{resolve_calendar_definition, ResolvedCalendar};
pub use error::{AppError, AppResult};
pub use hierarchy::{load_planet_hierarchy, LoadedPlanetHierarchy, SatelliteData};
pub use moon_phase::{
    compute_eclipse_proximity, compute_moon_phase, EclipseProximityInfo, LunarPhaseName,
    MoonPhaseInfo,
};
pub use season_tracking::{resolve_season_state, SeasonName, SeasonState};
pub use skeleton::{resolve_calendar_skeleton, CalendarSkeleton};
pub use tick::{
    cumulative_units_to_container, is_container_leap, resolve_calendar_tick,
    resolve_container_and_unit, units_in_container, CalendarTick,
};