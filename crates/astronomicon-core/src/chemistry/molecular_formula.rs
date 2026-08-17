use crate::chemistry::periodic_table::atomic_weight;
use crate::error::{DomainError, DomainResult};

pub fn parse(formula: &str) -> DomainResult<Vec<(String, u32)>> {
    let mut result = Vec::new();
    let chars: Vec<char> = formula.chars().collect();
    let mut i = 0;

    if chars.is_empty() {
        return Err(DomainError::InvalidInvariant {
            field: "formula".to_string(),
            reason: "cannot be empty".to_string(),
        });
    }

    while i < chars.len() {
        if !chars[i].is_ascii_uppercase() {
            return Err(DomainError::InvalidInvariant {
                field: "formula".to_string(),
                reason: format!("expected uppercase letter, found '{}'", chars[i]),
            });
        }

        let mut symbol = String::new();
        symbol.push(chars[i]);
        i += 1;

        if i < chars.len() && chars[i].is_ascii_lowercase() {
            symbol.push(chars[i]);
            i += 1;
        }

        if atomic_weight(&symbol).is_none() {
            return Err(DomainError::InvalidInvariant {
                field: "formula".to_string(),
                reason: format!("unknown element '{}'", symbol),
            });
        }

        let mut count_str = String::new();
        while i < chars.len() && chars[i].is_ascii_digit() {
            count_str.push(chars[i]);
            i += 1;
        }

        let count = if count_str.is_empty() {
            1
        } else {
            count_str
                .parse::<u32>()
                .map_err(|_| DomainError::InvalidInvariant {
                    field: "formula".to_string(),
                    reason: "invalid subscript".to_string(),
                })?
        };

        if count == 0 {
            return Err(DomainError::InvalidInvariant {
                field: "formula".to_string(),
                reason: "subscript cannot be zero".to_string(),
            });
        }

        result.push((symbol, count));
    }

    Ok(result)
}
