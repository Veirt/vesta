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
    fn name(&self) -> &'static str {
        "Clock"
    }

    fn description(&self) -> &'static str {
        "Shows the current time and date in a configurable time zone."
    }

    fn fields(&self) -> Vec<WidgetField> {
        vec![WidgetField {
            key: "time_zone",
            label: "Time zone",
            kind: "select",
            required: true,
            description: "Choose the IANA time zone used by this clock.",
            default: Some(serde_json::json!("UTC")),
            options: vec![
                WidgetOption {
                    value: "UTC",
                    label: "UTC",
                },
                WidgetOption {
                    value: "America/Los_Angeles",
                    label: "Los Angeles",
                },
                WidgetOption {
                    value: "America/Denver",
                    label: "Denver",
                },
                WidgetOption {
                    value: "America/Chicago",
                    label: "Chicago",
                },
                WidgetOption {
                    value: "America/New_York",
                    label: "New York",
                },
                WidgetOption {
                    value: "America/Sao_Paulo",
                    label: "São Paulo",
                },
                WidgetOption {
                    value: "Europe/London",
                    label: "London",
                },
                WidgetOption {
                    value: "Europe/Paris",
                    label: "Paris",
                },
                WidgetOption {
                    value: "Europe/Helsinki",
                    label: "Helsinki",
                },
                WidgetOption {
                    value: "Africa/Johannesburg",
                    label: "Johannesburg",
                },
                WidgetOption {
                    value: "Asia/Dubai",
                    label: "Dubai",
                },
                WidgetOption {
                    value: "Asia/Kolkata",
                    label: "Kolkata",
                },
                WidgetOption {
                    value: "Asia/Singapore",
                    label: "Singapore",
                },
                WidgetOption {
                    value: "Asia/Tokyo",
                    label: "Tokyo",
                },
                WidgetOption {
                    value: "Australia/Sydney",
                    label: "Sydney",
                },
            ],
        }]
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
