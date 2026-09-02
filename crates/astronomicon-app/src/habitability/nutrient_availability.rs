use crate::error::AppResult;
use crate::mineralogy::resolve_planetary_mineralogy;
use astronomicon_core::math::habitability::{
    evaluate_first_order_surface_nutrients, FirstOrderNutrientLimitation,
};
use astronomicon_core::units::Duration;
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::atmosphere_repository;
use uuid::Uuid;

pub async fn resolve_nutrient_limitation(
    pool: &SqlitePool,
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<FirstOrderNutrientLimitation> {
    let mineralogy =
        resolve_planetary_mineralogy(pool, planet_id, universe_epoch, at_epoch).await?;
    let atm_opt = atmosphere_repository::get_by_planet_id(pool, &planet_id).await?;

    let (n2_frac, co2_frac) = match atm_opt {
        Some(atm) => {
            let mut n2 = 0.0;
            let mut co2 = 0.0;
            for comp in atm.composition() {
                if comp.formula() == "N2" {
                    n2 += comp.percentage() / 100.0;
                } else if comp.formula() == "CO2" {
                    co2 += comp.percentage() / 100.0;
                }
            }
            (Some(n2), Some(co2))
        }
        None => (None, None),
    };

    Ok(evaluate_first_order_surface_nutrients(
        &mineralogy.abundance.crustal_abundances,
        n2_frac,
        co2_frac,
    ))
}
