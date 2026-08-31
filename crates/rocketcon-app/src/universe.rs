use crate::error::RocketResult;
use astronomicon_core::units::Duration;
use astronomicon_db::SqlitePool;

pub async fn resolve_universe_epoch(pool: &SqlitePool) -> RocketResult<Duration> {
    let state = astronomicon_app::universe::resolve_universe_state(pool).await?;
    Ok(state.elapsed_since_j2000())
}
