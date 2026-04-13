use std::sync::Arc;
use std::time::Duration;

use moka::future::Cache;
use reqwest::Client;

use crate::config::PingConfig;
use crate::error::VestaResult;

const DEFAULT_PING_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy)]
pub struct PingResult {
    pub is_up: bool,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct PingKey {
    pub group: String,
    pub title: String,
}

pub struct PingService {
    cache: Cache<PingKey, PingResult>,
    http_client: Client,
    timeout: Duration,
}

impl PingService {
    pub fn new(http_client: Client) -> Arc<Self> {
        let cache = Cache::builder()
            .max_capacity(1000)
            .time_to_live(DEFAULT_CACHE_TTL)
            .build();

        Arc::new(Self {
            cache,
            http_client,
            timeout: DEFAULT_PING_TIMEOUT,
        })
    }

    pub async fn check_service(&self, group: &str, title: &str, config: &PingConfig) -> VestaResult<bool> {
        let key = PingKey {
            group: group.to_string(),
            title: title.to_string(),
        };

        if let Some(cached) = self.cache.get(&key).await {
            return Ok(cached.is_up);
        }

        let result = self.perform_ping(config).await;
        let ping_result = PingResult {
            is_up: result.unwrap_or(false),
        };
        self.cache.insert(key, ping_result).await;
        Ok(ping_result.is_up)
    }

    async fn perform_ping(&self, config: &PingConfig) -> VestaResult<bool> {
        let response = self
            .http_client
            .get(&config.url)
            .timeout(self.timeout)
            .send()
            .await?;
        Ok(response.status().is_success())
    }
}
