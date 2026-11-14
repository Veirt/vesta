use async_trait::async_trait;
use maud::{Markup, html};
use serde::Deserialize;

use crate::config::Service;
use crate::error::VestaResult;
use crate::services::weather_service::{WeatherQuery, WeatherService};
use crate::widget_system::{
    WidgetContext, WidgetField, WidgetHandler, WidgetOption, WidgetRef, widget_scaffold,
};

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

fn temp_unit(units: &str) -> &'static str {
    if units == "fahrenheit" { "°F" } else { "°C" }
}

fn weather_body(config: &Config, data: &crate::services::weather_service::WeatherData) -> Markup {
    let unit = temp_unit(&config.units);
    let current = &data.current;
    html! {
        div class="weather-widget" {
            div class="text-center" {
                h3 class="widget-title" {
                    "Weather"
                }
                div class="widget-secondary" {
                    (format!("{:.2}°, {:.2}°", config.latitude, config.longitude))
                }
            }
            div class="flex items-center justify-between" {
                div class="flex-1" {
                    div class="weather-temperature widget-value" {
                        (format!("{:.0}{}", current.temperature_2m, unit))
                    }
                    div class="widget-secondary" {
                        "Feels " (format!("{:.0}{}", current.apparent_temperature, unit))
                    }
                    div class="weather-description" {
                        (description(current.weather_code))
                    }
                }
                div class="flex-shrink-0 text-3xl" {
                    (icon(current.weather_code))
                }
            }
            div class="weather-details widget-divider" {
                div class="text-center" {
                    div class="widget-secondary" { "Humidity" }
                    div class="weather-detail-value widget-value" {
                        (current.relative_humidity_2m) "%"
                    }
                }
                div class="text-center" {
                    div class="widget-secondary" { "Wind" }
                    div class="weather-detail-value widget-value" {
                        (format!("{:.0} {}", current.wind_speed_10m, wind_direction(current.wind_direction_10m)))
                    }
                }
            }
        }
    }
}

#[async_trait]
impl WidgetHandler for WeatherWidget {
    fn name(&self) -> &'static str {
        "Weather"
    }

    fn description(&self) -> &'static str {
        "Shows current weather for a latitude and longitude using Open-Meteo."
    }

    fn fields(&self) -> Vec<WidgetField> {
        vec![
            WidgetField {
                key: "latitude",
                label: "Latitude",
                kind: "number",
                required: true,
                description: "A value from −90 to 90.",
                default: None,
                options: Vec::new(),
            },
            WidgetField {
                key: "longitude",
                label: "Longitude",
                kind: "number",
                required: true,
                description: "A value from −180 to 180.",
                default: None,
                options: Vec::new(),
            },
            WidgetField {
                key: "units",
                label: "Temperature units",
                kind: "select",
                required: true,
                description: "Choose how temperatures are displayed.",
                default: Some(serde_json::json!("celsius")),
                options: vec![
                    WidgetOption {
                        value: "celsius",
                        label: "Celsius",
                    },
                    WidgetOption {
                        value: "fahrenheit",
                        label: "Fahrenheit",
                    },
                ],
            },
            WidgetField {
                key: "refresh_interval",
                label: "Refresh every (seconds)",
                kind: "number",
                required: true,
                description: "Choose a value from 60 to 3600 seconds.",
                default: Some(serde_json::json!(600)),
                options: Vec::new(),
            },
        ]
    }

    type Config = Config;

    fn render(&self, mount: &WidgetRef, service: &Service, config: &Config) -> Markup {
        widget_scaffold(service, mount, self.name(), Some(config.refresh_interval))
    }

    async fn handle_request(
        &self,
        _ctx: WidgetContext,
        _mount: WidgetRef,
        config: Config,
    ) -> VestaResult<Markup> {
        let data = self.weather.fetch(&config.query()).await?;
        Ok(weather_body(&config, &data))
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
