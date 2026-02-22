use async_trait::async_trait;
use maud::{Markup, html};
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    AppState,
    config::{Service, Widget},
    error::{VestaError, VestaResult},
    widget_system::{WidgetHandler, WidgetQuery},
    widgets::widget_container,
};

#[derive(Deserialize, Debug)]
pub struct WeatherConfig {
    pub latitude: f64,
    pub longitude: f64,
    #[serde(default = "default_units")]
    pub units: String, // celsius, fahrenheit
}

fn default_units() -> String {
    "celsius".to_string()
}

#[derive(Deserialize, Debug)]
struct OpenMeteoResponse {
    current: CurrentWeather,
}

#[derive(Deserialize, Debug)]
struct CurrentWeather {
    temperature_2m: f64,
    relative_humidity_2m: u32,
    apparent_temperature: f64,
    weather_code: u32,
    wind_speed_10m: f64,
    wind_direction_10m: u32,
}

pub struct WeatherWidget;

impl WeatherWidget {
    pub fn new() -> Self {
        Self
    }

    async fn fetch_weather(
        &self,
        client: &Client,
        config: &WeatherConfig,
    ) -> VestaResult<OpenMeteoResponse> {
        let temperature_unit = if config.units == "fahrenheit" {
            "fahrenheit"
        } else {
            "celsius"
        };

        let url = format!(
            "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=temperature_2m,relative_humidity_2m,apparent_temperature,weather_code,wind_speed_10m,wind_direction_10m&temperature_unit={}",
            config.latitude, config.longitude, temperature_unit
        );

        let response = client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(VestaError::ApiError {
                status: response.status(),
                message: "Failed to fetch weather data".to_string(),
            });
        }

        let weather_data = response.json::<OpenMeteoResponse>().await?;
        Ok(weather_data)
    }

    fn get_temperature_unit(&self, units: &str) -> &'static str {
        match units {
            "fahrenheit" => "°F",
            _ => "°C",
        }
    }

    fn get_weather_description(&self, weather_code: u32) -> &'static str {
        match weather_code {
            0 => "Clear sky",
            1 => "Mainly clear",
            2 => "Partly cloudy",
            3 => "Overcast",
            45 | 48 => "Fog",
            51 | 53 | 55 => "Drizzle",
            56 | 57 => "Freezing drizzle",
            61 | 63 | 65 => "Rain",
            66 | 67 => "Freezing rain",
            71 | 73 | 75 => "Snow",
            77 => "Snow grains",
            80..=82 => "Rain showers",
            85 | 86 => "Snow showers",
            95 => "Thunderstorm",
            96 | 99 => "Thunderstorm with hail",
            _ => "Unknown",
        }
    }

    fn get_weather_icon(&self, weather_code: u32) -> &'static str {
        match weather_code {
            0 => "☀️",
            1 => "🌤️",
            2 => "⛅",
            3 => "☁️",
            45 | 48 => "🌫️",
            51 | 53 | 55 => "🌦️",
            56 | 57 => "🌧️",
            61 | 63 | 65 => "🌧️",
            66 | 67 => "🌧️",
            71 | 73 | 75 => "🌨️",
            77 => "🌨️",
            80..=82 => "🌦️",
            85 | 86 => "🌨️",
            95 => "⛈️",
            96 | 99 => "⛈️",
            _ => "🌍",
        }
    }

    fn get_wind_direction(&self, degrees: u32) -> &'static str {
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
}

#[async_trait]
impl WidgetHandler for WeatherWidget {
    fn name(&self) -> &'static str {
        "Weather"
    }

    fn render(&self, group_id: &str, service: &Service) -> Markup {
        let refresh_interval = service
            .widget
            .as_ref()
            .and_then(|w| w.config.as_ref())
            .and_then(|c| c.get("refresh_interval"))
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(600);

        widget_container(
            service.width,
            service.height,
            "",
            html! {
                div
                    class="h-full"
                    hx-get=(format!("/api/widgets/Weather?group={}&title={}", group_id, service.title))
                    hx-trigger=(format!("load, every {}s", refresh_interval))
                    hx-swap="innerHTML" {
                    div class="flex items-center justify-center h-full" {
                        div class="animate-spin rounded-full h-6 w-6 border-b-2 border-violet-500" {}
                    }
                }
            },
        )
    }

    async fn handle_request(
        &self,
        state: Arc<AppState>,
        query: WidgetQuery,
    ) -> VestaResult<Markup> {
        let config_manager = &state.config_manager;
        let service = config_manager
            .get_service(&query.group, &query.title)?
            .ok_or_else(|| VestaError::Internal("Service not found".to_string()))?;

        let widget_config = service
            .widget
            .as_ref()
            .and_then(|w| w.config.as_ref())
            .ok_or_else(|| VestaError::Internal("Weather widget config not found".to_string()))?;

        let weather_config = WeatherConfig {
            latitude: widget_config
                .get("latitude")
                .ok_or_else(|| VestaError::Internal("latitude is required".to_string()))?
                .parse::<f64>()
                .map_err(|_| VestaError::Internal("latitude must be a valid number".to_string()))?,
            longitude: widget_config
                .get("longitude")
                .ok_or_else(|| VestaError::Internal("longitude is required".to_string()))?
                .parse::<f64>()
                .map_err(|_| {
                    VestaError::Internal("longitude must be a valid number".to_string())
                })?,
            units: widget_config
                .get("units")
                .unwrap_or(&"celsius".to_string())
                .to_string(),
        };

        let weather_data = self
            .fetch_weather(&state.http_client, &weather_config)
            .await?;
        let temp_unit = self.get_temperature_unit(&weather_config.units);
        let weather_description = self.get_weather_description(weather_data.current.weather_code);
        let weather_icon = self.get_weather_icon(weather_data.current.weather_code);
        let wind_direction = self.get_wind_direction(weather_data.current.wind_direction_10m);

        Ok(html! {
            div class="space-y-4" {
                // Header
                div class="text-center" {
                    h3 class="text-sm font-semibold text-zinc-100" style="font-family: 'JetBrains Mono', monospace;" {
                        "Weather"
                    }
                    div class="text-xs text-zinc-500 font-mono" {
                        (format!("{}°, {}°", weather_config.latitude, weather_config.longitude))
                    }
                }

                // Main weather display
                div class="flex items-center justify-between" {
                    div class="flex-1" {
                        div class="text-3xl font-bold text-zinc-100 mb-1 font-mono" {
                            (format!("{:.0}{}", weather_data.current.temperature_2m, temp_unit))
                        }
                        div class="text-xs text-zinc-500" {
                            "Feels " (format!("{:.0}{}", weather_data.current.apparent_temperature, temp_unit))
                        }
                        div class="text-sm text-zinc-400 capitalize mt-1" {
                            (weather_description)
                        }
                    }
                    div class="flex-shrink-0 text-3xl" {
                        (weather_icon)
                    }
                }

                // Additional details
                div class="grid grid-cols-2 gap-3 pt-3 border-t border-zinc-800" {
                    div class="text-center" {
                        div class="text-xs text-zinc-500 uppercase tracking-wide" { "Humidity" }
                        div class="text-sm font-semibold text-zinc-200 font-mono" { (weather_data.current.relative_humidity_2m) "%" }
                    }
                    div class="text-center" {
                        div class="text-xs text-zinc-500 uppercase tracking-wide" { "Wind" }
                        div class="text-sm font-semibold text-zinc-200 font-mono" {
                            (format!("{:.0} {}", weather_data.current.wind_speed_10m, wind_direction))
                        }
                    }
                }
            }
        })
    }

    fn validate_config(&self, widget: &Widget) -> VestaResult<()> {
        let config = widget
            .config
            .as_ref()
            .ok_or_else(|| VestaError::Internal("Weather widget requires config".to_string()))?;

        if config.get("latitude").is_none() {
            return Err(VestaError::Internal("latitude is required".to_string()));
        }

        if config.get("longitude").is_none() {
            return Err(VestaError::Internal("longitude is required".to_string()));
        }

        if let Some(lat_str) = config.get("latitude") {
            if let Ok(lat) = lat_str.parse::<f64>() {
                if !(-90.0..=90.0).contains(&lat) {
                    return Err(VestaError::Internal(
                        "latitude must be between -90 and 90".to_string(),
                    ));
                }
            } else {
                return Err(VestaError::Internal(
                    "latitude must be a valid number".to_string(),
                ));
            }
        }

        if let Some(lon_str) = config.get("longitude") {
            if let Ok(lon) = lon_str.parse::<f64>() {
                if !(-180.0..=180.0).contains(&lon) {
                    return Err(VestaError::Internal(
                        "longitude must be between -180 and 180".to_string(),
                    ));
                }
            } else {
                return Err(VestaError::Internal(
                    "longitude must be a valid number".to_string(),
                ));
            }
        }

        if let Some(units) = config.get("units")
            && !["celsius", "fahrenheit"].contains(&units.as_str())
        {
            return Err(VestaError::Internal(
                "units must be 'celsius' or 'fahrenheit'".to_string(),
            ));
        }

        if let Some(interval_str) = config.get("refresh_interval")
            && let Ok(interval) = interval_str.parse::<u64>()
            && !(60..=3600).contains(&interval)
        {
            return Err(VestaError::Internal(
                "refresh_interval must be between 60 and 3600 seconds".to_string(),
            ));
        }

        Ok(())
    }
}
