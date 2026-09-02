use crate::error::RocketResult;
use astronomicon_app::AppContext;

pub async fn build_context() -> RocketResult<AppContext> {
    let pool = rocketcon_db::save::resolve_current_save_pool().await?;
    Ok(AppContext::new(pool))
}