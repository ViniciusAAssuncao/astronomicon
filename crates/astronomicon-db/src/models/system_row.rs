use crate::error::DbError;
use astronomicon_core::domain::StarSystem;
use astronomicon_core::units::{Angle, Length};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct StarSystemRow {
    pub id: String,
    pub name: String,
    pub right_ascension_rad: Option<f64>,
    pub declination_rad: Option<f64>,
    pub distance_from_sol_m: Option<f64>,
}

impl TryFrom<StarSystemRow> for StarSystem {
    type Error = DbError;

    fn try_from(row: StarSystemRow) -> Result<Self, Self::Error> {
        let id = Uuid::parse_str(&row.id)?;
        let right_ascension = row.right_ascension_rad.map(Angle::new);
        let declination = row.declination_rad.map(Angle::new);
        let distance_from_sun = row.distance_from_sol_m.map(Length::new);

        let system = StarSystem::new(id, row.name, right_ascension, declination, distance_from_sun)?;
        Ok(system)
    }
}
