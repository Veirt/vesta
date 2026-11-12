//! Serde helpers for widget configs.
//!
//! Every value in `widget.config` is a TOML string (`refresh_interval = "5"`),
//! but widget `Config` structs want typed numbers. These deserializers accept
//! both a native number and a string that parses, so configs read naturally
//! in either style.

use serde::{Deserialize, Deserializer};

fn number_from<'de, T: serde::de::DeserializeOwned, D>(
    deserializer: D,
    parse: fn(&str) -> Option<T>,
) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match &value {
        serde_json::Value::Number(_) => T::deserialize(value).map_err(serde::de::Error::custom),
        serde_json::Value::String(s) => {
            parse(s).ok_or_else(|| serde::de::Error::custom(format!("'{s}' is not a valid number")))
        }
        other => Err(serde::de::Error::custom(format!(
            "expected a number, got {other}"
        ))),
    }
}

/// Deserializes u64 from a JSON number or a string that parses as u64.
pub fn debounced_u64<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    number_from(deserializer, |s| s.parse().ok())
}

/// Deserializes f64 from a JSON number or a string that parses as f64.
pub fn debounced_f64<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
    number_from(deserializer, |s| s.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::{debounced_f64, debounced_u64};
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct Sample {
        #[serde(deserialize_with = "debounced_u64")]
        count: u64,
        #[serde(deserialize_with = "debounced_f64")]
        ratio: f64,
    }

    #[test]
    fn accepts_native_numbers() {
        let sample: Sample = serde_json::from_value(serde_json::json!({
            "count": 5, "ratio": 0.5
        }))
        .unwrap();
        assert_eq!(sample.count, 5);
        assert_eq!(sample.ratio, 0.5);
    }

    #[test]
    fn accepts_strings_that_parse() {
        let sample: Sample = serde_json::from_value(serde_json::json!({
            "count": "5", "ratio": "0.5"
        }))
        .unwrap();
        assert_eq!(sample.count, 5);
        assert_eq!(sample.ratio, 0.5);
    }

    #[test]
    fn rejects_non_numeric_strings() {
        let result = serde_json::from_value::<Sample>(serde_json::json!({
            "count": "five", "ratio": 1.0
        }));
        assert!(result.is_err());
    }
}
