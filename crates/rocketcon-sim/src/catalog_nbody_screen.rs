use rocketcon_core::math::orbital::n_body::dop853::Dop853Error;
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub struct GravityInfluence {
    pub body_id: Uuid,
    pub acceleration_m_s2: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct HorizonScreen {
    pub instantaneous_omitted_acceleration_m_s2: f64,
    pub constant_acceleration_displacement_m: f64,
    pub within_experimental_budget: bool,
}

pub fn screen_omitted_gravity(
    influences: &[GravityInfluence],
    included: &[Uuid],
    remaining_horizon_s: f64,
    comparison_budget_m: f64,
) -> Result<HorizonScreen, Dop853Error> {
    if influences.is_empty()
        || included.is_empty()
        || !remaining_horizon_s.is_finite()
        || remaining_horizon_s <= 0.0
        || !comparison_budget_m.is_finite()
        || comparison_budget_m <= 0.0
        || included
            .iter()
            .enumerate()
            .any(|(i, id)| included[..i].contains(id))
        || influences.iter().enumerate().any(|(i, source)| {
            !source.acceleration_m_s2.is_finite()
                || source.acceleration_m_s2 <= 0.0
                || influences[..i]
                    .iter()
                    .any(|other| other.body_id == source.body_id)
        })
        || included
            .iter()
            .any(|id| !influences.iter().any(|s| s.body_id == *id))
    {
        return Err(Dop853Error::InvalidGravitySource);
    }
    let omitted = influences
        .iter()
        .filter(|s| !included.contains(&s.body_id))
        .map(|s| s.acceleration_m_s2)
        .sum::<f64>();
    let indicator = 0.5 * omitted * remaining_horizon_s * remaining_horizon_s;
    if !omitted.is_finite() || !indicator.is_finite() {
        return Err(Dop853Error::NumericalBreakdown);
    }
    Ok(HorizonScreen {
        instantaneous_omitted_acceleration_m_s2: omitted,
        constant_acceleration_displacement_m: indicator,
        within_experimental_budget: indicator <= comparison_budget_m,
    })
}