use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::error::VestaResult;
use crate::services::weather_service::{WeatherQuery, WeatherService};
use crate::widget_system::{WidgetContext, WidgetField, WidgetHandler, WidgetOption, WidgetQuery};

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    #[serde(deserialize_with = "crate::widget_config::debounced_f64")]
    pub latitude: f64,
    #[serde(deserialize_with = "crate::widget_config::debounced_f64")]
    pub longitude: f64,
    /// "celsius" (default) or "fahrenheit".
    #[serde(default = "default_units")]
    pub units: String,
    /// Refresh cadence in seconds (60–3600).
    #[serde(
        default = "default_refresh",
        deserialize_with = "crate::widget_config::debounced_u64"
    )]
    pub refresh_interval: u64,
}

fn default_units() -> String {
    "celsius".to_string()
}

fn default_refresh() -> u64 {
    600
}

impl Config {
    fn query(&self) -> WeatherQuery {
        WeatherQuery {
            latitude: self.latitude,
            longitude: self.longitude,
            units: self.units.clone(),
        }
    }
}

pub struct WeatherWidget {
    weather: WeatherService,
}

impl WeatherWidget {
    pub fn new(http_client: reqwest::Client) -> Self {
        Self {
            weather: WeatherService::new(http_client),
        }
    }
}

fn description(code: u32) -> &'static str {
    match code {
        0 => "Clear sky",
        1 => "Mainly clear",
        2 => "Partly cloudy",
        3 => "Overcast",
        45 | 48 => "Fog",
        51..=55 => "Drizzle",
        56 | 57 => "Freezing drizzle",
        61..=65 => "Rain",
        66 | 67 => "Freezing rain",
        71..=75 => "Snow",
        77 => "Snow grains",
        80..=82 => "Rain showers",
        85 | 86 => "Snow showers",
        95 => "Thunderstorm",
        96 | 99 => "Thunderstorm with hail",
        _ => "Unknown",
    }
}

fn icon(code: u32) -> &'static str {
    match code {
        0 => "☀️",
        1 => "🌤️",
        2 => "⛅",
        3 => "☁️",
        45 | 48 => "🌫️",
        51..=57 | 61..=67 | 80..=82 => "🌧️",
        71..=77 | 85 | 86 => "🌨️",
        95 | 96 | 99 => "⛈️",
        _ => "🌍",
    }
}

fn wind_direction(degrees: u32) -> &'static str {
    match degrees {
        0..=22 | 338..=360 => "N",
        23..=67 => "NE",
        68..=112 => "E",
        113..=157 => "SE",
        158..=202 => "S",
        203..=247 => "SW",
        248..=292 => "W",
        293..=337 => "NW",
        _ => "N",
    }
}

#[async_trait]
impl WidgetHandler for WeatherWidget {
    fn name(&self) -> &str {
        "Weather"
    }

    fn description(&self) -> &str {
        "Shows current weather for a latitude and longitude using Open-Meteo."
    }

    fn default_size(&self) -> (u8, u8) {
        (4, 3)
    }

    fn min_size(&self) -> (u8, u8) {
        (3, 3)
    }

    fn fields(&self) -> Vec<WidgetField> {
        vec![
            WidgetField::builtin(
                "latitude",
                "Latitude",
                "number",
                true,
                "A value from −90 to 90.",
            )
            .with_range(-90.0, 90.0),
            WidgetField::builtin(
                "longitude",
                "Longitude",
                "number",
                true,
                "A value from −180 to 180.",
            )
            .with_range(-180.0, 180.0),
            WidgetField::builtin(
                "units",
                "Temperature units",
                "select",
                true,
                "Choose how temperatures are displayed.",
            )
            .with_default(serde_json::json!("celsius"))
            .with_options(vec![
                WidgetOption::builtin("celsius", "Celsius"),
                WidgetOption::builtin("fahrenheit", "Fahrenheit"),
            ]),
            WidgetField::builtin(
                "refresh_interval",
                "Refresh every (seconds)",
                "number",
                true,
                "Choose a value from 60 to 3600 seconds.",
            )
            .with_default(serde_json::json!(600))
            .with_range(60.0, 3600.0),
        ]
    }

    type Config = Config;

    fn refresh_interval_secs(&self, config: &Config) -> Option<u64> {
        Some(config.refresh_interval)
    }

    async fn handle_request(
        &self,
        _ctx: WidgetContext,
        _query: WidgetQuery,
        config: Config,
    ) -> VestaResult<Value> {
        let data = self.weather.fetch(&config.query()).await?;
        let current = data.current;
        Ok(json!({
            "latitude": config.latitude,
            "longitude": config.longitude,
            "units": config.units,
            "current": {
                "temperature": current.temperature_2m,
                "apparent_temperature": current.apparent_temperature,
                "relative_humidity": current.relative_humidity_2m,
                "weather_code": current.weather_code,
                "wind_speed": current.wind_speed_10m,
                "wind_direction_degrees": current.wind_direction_10m,
                "description": description(current.weather_code),
                "icon": icon(current.weather_code),
                "wind_direction": wind_direction(current.wind_direction_10m),
            },
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_defaults_match_previous_behavior() {
        let config: Config = serde_json::from_value(serde_json::json!({
            "latitude": "0.5016", "longitude": "117.1265"
        }))
        .unwrap();
        assert_eq!(config.units, "celsius");
        assert_eq!(config.refresh_interval, 600);
    }

    #[test]
    fn weather_codes_map_to_description_and_icon() {
        assert_eq!(description(0), "Clear sky");
        assert_eq!(description(95), "Thunderstorm");
        assert_eq!(description(9999), "Unknown");
        assert_eq!(icon(0), "☀️");
    }

    #[test]
    fn wind_directions_cover_compass() {
        assert_eq!(wind_direction(0), "N");
        assert_eq!(wind_direction(180), "S");
        assert_eq!(wind_direction(270), "W");
    }
}
