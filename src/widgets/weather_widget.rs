use async_trait::async_trait;
use maud::{html, Markup};
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    config::{Service, Widget},
    error::{VestaError, VestaResult},
    widget_system::{WidgetHandler, WidgetQuery},
    AppState,
};

#[derive(Deserialize, Debug)]
pub struct WeatherConfig {
    pub latitude: f64,
    pub longitude: f64,
    #[serde(default = "default_units")]
    pub units: String, // celsius, fahrenheit
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval: u64, // in seconds
}

fn default_units() -> String {
    "celsius".to_string()
}

fn default_refresh_interval() -> u64 {
    600 // 10 minutes
}

#[derive(Deserialize, Debug)]
struct OpenMeteoResponse {
    current: CurrentWeather,
    #[serde(rename = "current_units")]
    current_units: CurrentUnits,
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

#[derive(Deserialize, Debug)]
struct CurrentUnits {
    temperature_2m: String,
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
            80 | 81 | 82 => "Rain showers",
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
            80 | 81 | 82 => "🌦️",
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
        let config = service.widget.as_ref().and_then(|w| w.config.as_ref());

        let refresh_interval = config
            .and_then(|c| c.get("refresh_interval"))
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(600);

        let width = service.width.unwrap_or(1);
        let height = service.height.unwrap_or(1);

        html! {
            div class=(format!("bg-slate-900 border border-slate-800 rounded-xl p-4 h-full"))
                style=(format!("grid-column: span {} / span {}; grid-row: span {} / span {};", width, width, height, height))
                 hx-get=(format!("/api/widgets/Weather?group={}&title={}", group_id, service.title))
                 hx-trigger=(format!("load, every {}s", refresh_interval))
                 hx-swap="innerHTML" {
                div class="flex items-center justify-center h-full" {
                    div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500" {}
                }
            }
        }
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
            refresh_interval: widget_config
                .get("refresh_interval")
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(600),
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
                    h3 class="text-lg font-semibold text-white" {
                        "Weather"
                    }
                    div class="text-xs text-gray-400" {
                        (format!("{}°, {}°", weather_config.latitude, weather_config.longitude))
                    }
                }

                // Main weather display
                div class="flex items-center justify-between" {
                    div class="flex-1" {
                        div class="text-3xl font-bold text-white mb-1" {
                            (format!("{:.0}{}", weather_data.current.temperature_2m, temp_unit))
                        }
                        div class="text-sm text-gray-400" {
                            "Feels like " (format!("{:.0}{}", weather_data.current.apparent_temperature, temp_unit))
                        }
                        div class="text-sm text-gray-300 capitalize mt-2" {
                            (weather_description)
                        }
                    }
                    div class="flex-shrink-0 text-4xl" {
                        (weather_icon)
                    }
                }

                // Additional details
                div class="grid grid-cols-2 gap-4 pt-2 border-t border-gray-700" {
                    div class="text-center" {
                        div class="text-xs text-gray-400" { "Humidity" }
                        div class="text-sm font-semibold text-white" { (weather_data.current.relative_humidity_2m) "%" }
                    }
                    div class="text-center" {
                        div class="text-xs text-gray-400" { "Wind" }
                        div class="text-sm font-semibold text-white" {
                            (format!("{:.0} km/h {}", weather_data.current.wind_speed_10m, wind_direction))
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

        // Validate required fields
        if config.get("latitude").is_none() {
            return Err(VestaError::Internal("latitude is required".to_string()));
        }

        if config.get("longitude").is_none() {
            return Err(VestaError::Internal("longitude is required".to_string()));
        }

        // Validate latitude
        if let Some(lat_str) = config.get("latitude") {
            if let Ok(lat) = lat_str.parse::<f64>() {
                if lat < -90.0 || lat > 90.0 {
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

        // Validate longitude
        if let Some(lon_str) = config.get("longitude") {
            if let Ok(lon) = lon_str.parse::<f64>() {
                if lon < -180.0 || lon > 180.0 {
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

        // Validate units if provided
        if let Some(units) = config.get("units") {
            if !["celsius", "fahrenheit"].contains(&units.as_str()) {
                return Err(VestaError::Internal(
                    "units must be 'celsius' or 'fahrenheit'".to_string(),
                ));
            }
        }

        // Validate refresh interval if provided
        if let Some(interval_str) = config.get("refresh_interval") {
            if let Ok(interval) = interval_str.parse::<u64>() {
                if interval < 60 || interval > 3600 {
                    return Err(VestaError::Internal(
                        "refresh_interval must be between 60 and 3600 seconds".to_string(),
                    ));
                }
            } else {
                return Err(VestaError::Internal(
                    "refresh_interval must be a number".to_string(),
                ));
            }
        }

        Ok(())
    }
}
