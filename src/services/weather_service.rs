use std::sync::Arc;
use std::time::Duration;

use moka::future::Cache;
use reqwest::Client;
use serde::Deserialize;

use crate::error::{VestaError, VestaResult};

const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(600);

#[derive(Deserialize, Debug)]
pub struct WeatherConfig {
    pub latitude: f64,
    pub longitude: f64,
    pub units: String,
}

impl WeatherConfig {
    pub fn cache_key(&self) -> String {
        format!("{}:{}:{}", self.latitude, self.longitude, self.units)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct CurrentWeather {
    pub temperature_2m: f64,
    pub relative_humidity_2m: u32,
    pub apparent_temperature: f64,
    pub weather_code: u32,
    pub wind_speed_10m: f64,
    pub wind_direction_10m: u32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct WeatherData {
    pub current: CurrentWeather,
}

pub struct WeatherService {
    cache: Cache<String, WeatherData>,
    http_client: Client,
}

impl WeatherService {
    pub fn new(http_client: Client) -> Arc<Self> {
        let cache = Cache::builder()
            .max_capacity(100)
            .time_to_live(DEFAULT_CACHE_TTL)
            .build();

        Arc::new(Self {
            cache,
            http_client,
        })
    }

    pub async fn fetch_weather(&self, config: &WeatherConfig) -> VestaResult<WeatherData> {
        let cache_key = config.cache_key();

        if let Some(cached) = self.cache.get(&cache_key).await {
            return Ok(cached);
        }

        let weather_data = self.fetch_from_api(config).await?;
        self.cache.insert(cache_key, weather_data.clone()).await;
        Ok(weather_data)
    }

    async fn fetch_from_api(&self, config: &WeatherConfig) -> VestaResult<WeatherData> {
        let temperature_unit = if config.units == "fahrenheit" {
            "fahrenheit"
        } else {
            "celsius"
        };

        let url = format!(
            "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=temperature_2m,relative_humidity_2m,apparent_temperature,weather_code,wind_speed_10m,wind_direction_10m&temperature_unit={}",
            config.latitude, config.longitude, temperature_unit
        );

        let response = self.http_client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(VestaError::ApiError {
                status: response.status(),
                message: "Failed to fetch weather data".to_string(),
            });
        }

        let weather_data = response.json::<WeatherData>().await?;
        Ok(weather_data)
    }
}
