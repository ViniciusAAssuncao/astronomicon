use crate::error::DbError;
use astronomicon_core::domain::UniverseState;
use astronomicon_core::units::Duration;
use sqlx::FromRow;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct UniverseStateRow {
    pub id: i64,
    pub seconds_since_j2000_epoch: f64,
}

impl TryFrom<UniverseStateRow> for UniverseState {
    type Error = DbError;

    fn try_from(row: UniverseStateRow) -> Result<Self, Self::Error> {
        let state = UniverseState::new(Duration::new(row.seconds_since_j2000_epoch))?;
        Ok(state)
    }
}
