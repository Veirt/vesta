use std::time::Duration;

use moka::future::Cache;
use reqwest::Client;

use crate::config::PingConfig;
use crate::error::VestaResult;

const PING_TIMEOUT: Duration = Duration::from_secs(5);
const CACHE_TTL: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy)]
pub struct PingResult {
    pub is_up: bool,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct PingKey {
    pub id: String,
    pub url: String,
}

pub struct PingService {
    cache: Cache<PingKey, PingResult>,
    http_client: Client,
    timeout: Duration,
}

impl PingService {
    pub fn new(http_client: Client) -> Self {
        let cache = Cache::builder()
            .max_capacity(1000)
            .time_to_live(CACHE_TTL)
            .build();
        Self {
            cache,
            http_client,
            timeout: PING_TIMEOUT,
        }
    }

    pub async fn check(&self, key: &PingKey, config: &PingConfig) -> bool {
        if let Some(cached) = self.cache.get(key).await {
            return cached.is_up;
        }

        let is_up = self.perform_ping(config).await.unwrap_or(false);
        self.cache.insert(key.clone(), PingResult { is_up }).await;
        is_up
    }

    pub async fn test_url(&self, url: &str) -> bool {
        self.perform_ping(&PingConfig {
            url: url.to_string(),
        })
        .await
        .unwrap_or(false)
    }

    async fn perform_ping(&self, config: &PingConfig) -> VestaResult<bool> {
        let url = reqwest::Url::parse(&config.url)
            .map_err(|e| crate::error::VestaError::InvalidWidgetConfig(format!("ping url: {e}")))?;

        let head = self
            .http_client
            .head(url.clone())
            .timeout(self.timeout)
            .send()
            .await;
        if let Ok(response) = &head {
            if response.status().is_success() {
                return Ok(true);
            }
            if response.status() != reqwest::StatusCode::METHOD_NOT_ALLOWED {
                return Ok(false);
            }
        }

        let response = self
            .http_client
            .get(url)
            .timeout(self.timeout)
            .send()
            .await?;
        Ok(response.status().is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cached_result_short_circuits_and_keys_by_url() {
        let service = PingService::new(Client::new());
        let cached_key = PingKey {
            id: "service".into(),
            url: "http://127.0.0.1:1".into(),
        };
        service
            .cache
            .insert(cached_key.clone(), PingResult { is_up: true })
            .await;
        let config = PingConfig {
            url: "http://127.0.0.1:1".to_string(),
        };
        assert!(service.check(&cached_key, &config).await);

        let updated_url_key = PingKey {
            id: "service".into(),
            url: "http://127.0.0.1:2".into(),
        };
        let updated_config = PingConfig {
            url: "http://127.0.0.1:2".to_string(),
        };
        assert!(!service.check(&updated_url_key, &updated_config).await);
    }
}
