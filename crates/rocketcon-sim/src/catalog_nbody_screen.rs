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

#[cfg(test)]
mod tests {
    use super::*;

    fn sources() -> [GravityInfluence; 3] {
        [
            GravityInfluence {
                body_id: Uuid::from_u128(1),
                acceleration_m_s2: 0.2,
            },
            GravityInfluence {
                body_id: Uuid::from_u128(2),
                acceleration_m_s2: 0.01,
            },
            GravityInfluence {
                body_id: Uuid::from_u128(3),
                acceleration_m_s2: 0.001,
            },
        ]
    }

    #[test]
    fn horizon_changes_the_experimental_screen() {
        let items = sources();
        let keep = [items[0].body_id, items[1].body_id];
        let short = screen_omitted_gravity(&items, &keep, 10.0, 1.0).unwrap();
        let long = screen_omitted_gravity(&items, &keep, 100.0, 1.0).unwrap();
        assert!((short.constant_acceleration_displacement_m - 0.05).abs() < 1e-12);
        assert!(short.within_experimental_budget);
        assert!(!long.within_experimental_budget);
        assert!((long.constant_acceleration_displacement_m - 5.0).abs() < 1e-12);
    }

    #[test]
    fn all_sources_have_zero_omitted_acceleration() {
        let items = sources();
        let selected = items.map(|item| item.body_id);
        let result = screen_omitted_gravity(&items, &selected, 86400.0, 1.0).unwrap();
        assert_eq!(result.instantaneous_omitted_acceleration_m_s2, 0.0);
        assert_eq!(result.constant_acceleration_displacement_m, 0.0);
        assert!(result.within_experimental_budget);
    }

    #[test]
    fn rejects_missing_duplicate_and_invalid_parameters() {
        let items = sources();
        let id = items[0].body_id;
        assert!(screen_omitted_gravity(&items, &[], 10.0, 1.0).is_err());
        assert!(screen_omitted_gravity(&items, &[id, id], 10.0, 1.0).is_err());
        assert!(screen_omitted_gravity(&items, &[Uuid::from_u128(9)], 10.0, 1.0).is_err());
        assert!(screen_omitted_gravity(&items, &[id], 0.0, 1.0).is_err());
        assert!(screen_omitted_gravity(&items, &[id], 10.0, f64::NAN).is_err());
        assert!(screen_omitted_gravity(&items, &[id], f64::MAX, 1.0).is_err());
    }
}
