use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::error::VestaResult;
use crate::widget_system::{WidgetContext, WidgetField, WidgetHandler, WidgetOption, WidgetQuery};

/// The React client owns the ticking display and formats it in this IANA zone.
#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_time_zone")]
    pub time_zone: String,
}

fn default_time_zone() -> String {
    "UTC".to_string()
}

pub struct ClockWidget;

impl ClockWidget {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ClockWidget {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WidgetHandler for ClockWidget {
    fn name(&self) -> &str {
        "Clock"
    }

    fn description(&self) -> &str {
        "Shows the current time and date in a configurable time zone."
    }

    fn fields(&self) -> Vec<WidgetField> {
        vec![
            WidgetField::builtin(
                "time_zone",
                "Time zone",
                "select",
                true,
                "Choose the IANA time zone used by this clock.",
            )
            .with_default(serde_json::json!("UTC"))
            .with_options(vec![
                WidgetOption::builtin("UTC", "UTC"),
                WidgetOption::builtin("America/Los_Angeles", "Los Angeles"),
                WidgetOption::builtin("America/Denver", "Denver"),
                WidgetOption::builtin("America/Chicago", "Chicago"),
                WidgetOption::builtin("America/New_York", "New York"),
                WidgetOption::builtin("America/Sao_Paulo", "São Paulo"),
                WidgetOption::builtin("Europe/London", "London"),
                WidgetOption::builtin("Europe/Paris", "Paris"),
                WidgetOption::builtin("Europe/Helsinki", "Helsinki"),
                WidgetOption::builtin("Africa/Johannesburg", "Johannesburg"),
                WidgetOption::builtin("Asia/Dubai", "Dubai"),
                WidgetOption::builtin("Asia/Kolkata", "Kolkata"),
                WidgetOption::builtin("Asia/Singapore", "Singapore"),
                WidgetOption::builtin("Asia/Tokyo", "Tokyo"),
                WidgetOption::builtin("Australia/Sydney", "Sydney"),
            ]),
        ]
    }

    fn default_size(&self) -> (u8, u8) {
        (4, 2)
    }

    fn min_size(&self) -> (u8, u8) {
        (3, 2)
    }

    type Config = Config;

    async fn handle_request(
        &self,
        _ctx: WidgetContext,
        _query: WidgetQuery,
        config: Config,
    ) -> VestaResult<Value> {
        Ok(json!({
            "timestamp": Utc::now().to_rfc3339(),
            "time_zone": config.time_zone,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_defaults_to_utc_and_accepts_selected_zone() {
        let default: Config = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(default.time_zone, "UTC");
        let configured: Config =
            serde_json::from_value(serde_json::json!({"time_zone": "Asia/Tokyo"})).unwrap();
        assert_eq!(configured.time_zone, "Asia/Tokyo");
    }
}
