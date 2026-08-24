use crate::error::{DomainError, DomainResult};

pub fn total_composition_weight(composition: &[(String, f64)]) -> f64 {
    composition.iter().map(|(_, p)| p).sum()
}

pub fn normalize_composition_fractions<'a>(
    composition: &'a [(String, f64)],
) -> Option<Vec<(&'a str, f64)>> {
    let total = total_composition_weight(composition);
    if total <= 0.0 {
        return None;
    }
    Some(
        composition
            .iter()
            .map(|(formula, amount)| (formula.as_str(), amount / total))
            .collect(),
    )
}

pub fn validate_and_normalize_composition<'a>(
    composition: &'a [(String, f64)],
    field: &str,
    reason: &str,
) -> DomainResult<Vec<(&'a str, f64)>> {
    let total = total_composition_weight(composition);
    if total <= 0.0 {
        return Err(DomainError::InvalidInvariant {
            field: field.to_string(),
            reason: reason.to_string(),
        });
    }
    Ok(composition
        .iter()
        .map(|(formula, amount)| (formula.as_str(), amount / total))
        .collect())
}

pub fn composition_weighted_mean_or_zero<F>(
    composition: &[(String, f64)],
    mut fetch: F,
) -> DomainResult<f64>
where
    F: FnMut(&str) -> DomainResult<f64>,
{
    let fractions = match normalize_composition_fractions(composition) {
        Some(f) => f,
        None => return Ok(0.0),
    };

    let mut sum = 0.0;
    for (formula, fraction) in fractions {
        let value = fetch(formula)?;
        sum += value * fraction;
    }

    Ok(sum)
}

pub fn composition_weighted_mean_strictly_positive<F>(
    composition: &[(String, f64)],
    field: &str,
    reason: &str,
    mut fetch: F,
) -> DomainResult<f64>
where
    F: FnMut(&str) -> DomainResult<f64>,
{
    let fractions = validate_and_normalize_composition(composition, field, reason)?;

    let mut sum = 0.0;
    for (formula, fraction) in fractions {
        let value = fetch(formula)?;
        sum += value * fraction;
    }

    Ok(sum)
}
