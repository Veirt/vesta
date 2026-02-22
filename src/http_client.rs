use reqwest::Client;
use std::time::Duration;

use crate::error::{VestaError, VestaResult};

struct HttpClientBuilder {
    timeout: Duration,
    connect_timeout: Duration,
    accept_invalid_certs: bool,
    pool_idle_timeout: Duration,
    pool_max_idle_per_host: usize,
}

impl Default for HttpClientBuilder {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(10),
            accept_invalid_certs: true,
            pool_idle_timeout: Duration::from_secs(30),
            pool_max_idle_per_host: 10,
        }
    }
}

impl HttpClientBuilder {
    fn build(self) -> VestaResult<Client> {
        let mut builder = Client::builder()
            .timeout(self.timeout)
            .connect_timeout(self.connect_timeout)
            .pool_idle_timeout(self.pool_idle_timeout)
            .pool_max_idle_per_host(self.pool_max_idle_per_host);

        if self.accept_invalid_certs {
            builder = builder.danger_accept_invalid_certs(true);
        }

        builder
            .build()
            .map_err(|e| VestaError::Internal(format!("Failed to create HTTP client: {}", e)))
    }
}

pub fn create_default_client() -> VestaResult<Client> {
    HttpClientBuilder::default().build()
}
