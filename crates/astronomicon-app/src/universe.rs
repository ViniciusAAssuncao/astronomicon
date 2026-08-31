use crate::error::AppResult;
use astronomicon_core::domain::UniverseState;
use astronomicon_core::error::DomainError;
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::universe_state_repository;

pub async fn resolve_universe_state(pool: &SqlitePool) -> AppResult<UniverseState> {
    let row = universe_state_repository::get(pool)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "universe_state".to_string(),
            reason: "universe state not found".to_string(),
        })?;
    let state = UniverseState::try_from(row)?;
    Ok(state)
}