use std::time::Duration;

use moka::future::Cache;
use reqwest::Client;
use serde::Deserialize;

use crate::error::VestaResult;
use crate::http_client::send_checked_json;

const CACHE_TTL: Duration = Duration::from_secs(600);

#[derive(Clone, Debug, Deserialize)]
pub struct WeatherQuery {
    #[serde(deserialize_with = "crate::widget_config::debounced_f64")]
    pub latitude: f64,
    #[serde(deserialize_with = "crate::widget_config::debounced_f64")]
    pub longitude: f64,
    /// "celsius" (default) or "fahrenheit".
    #[serde(default = "default_units")]
    pub units: String,
}

fn default_units() -> String {
    "celsius".to_string()
}

impl WeatherQuery {
    fn cache_key(&self) -> String {
        format!("{}:{}:{}", self.latitude, self.longitude, self.units)
    }

    fn temperature_unit(&self) -> &'static str {
        if self.units == "fahrenheit" {
            "fahrenheit"
        } else {
            "celsius"
        }
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
    pub fn new(http_client: Client) -> Self {
        let cache = Cache::builder()
            .max_capacity(100)
            .time_to_live(CACHE_TTL)
            .build();
        Self { cache, http_client }
    }

    pub async fn fetch(&self, query: &WeatherQuery) -> VestaResult<WeatherData> {
        let key = query.cache_key();
        if let Some(cached) = self.cache.get(&key).await {
            return Ok(cached);
        }
        let data = self.fetch_from_api(query).await?;
        self.cache.insert(key, data.clone()).await;
        Ok(data)
    }

    async fn fetch_from_api(&self, query: &WeatherQuery) -> VestaResult<WeatherData> {
        let url = format!(
            "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=temperature_2m,relative_humidity_2m,apparent_temperature,weather_code,wind_speed_10m,wind_direction_10m&temperature_unit={}",
            query.latitude,
            query.longitude,
            query.temperature_unit()
        );

        send_checked_json(self.http_client.get(&url), |_| {
            "weather provider error".to_string()
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_celsius() {
        let query: WeatherQuery = serde_json::from_value(serde_json::json!({
            "latitude": "0.5", "longitude": "117.1"
        }))
        .unwrap();
        assert_eq!(query.units, "celsius");
        assert_eq!(query.temperature_unit(), "celsius");
    }

    #[test]
    fn cache_key_distinguishes_units() {
        let c: WeatherQuery = serde_json::from_value(
            serde_json::json!({"latitude": "1", "longitude": "2", "units": "celsius"}),
        )
        .unwrap();
        let f: WeatherQuery = serde_json::from_value(
            serde_json::json!({"latitude": "1", "longitude": "2", "units": "fahrenheit"}),
        )
        .unwrap();
        assert_ne!(c.cache_key(), f.cache_key());
    }
}
