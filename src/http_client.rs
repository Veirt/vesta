use reqwest::Client;
use std::time::Duration;

use crate::error::{VestaError, VestaResult};

pub struct HttpClientBuilder {
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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn connect_timeout(mut self, connect_timeout: Duration) -> Self {
        self.connect_timeout = connect_timeout;
        self
    }

    pub fn accept_invalid_certs(mut self, accept: bool) -> Self {
        self.accept_invalid_certs = accept;
        self
    }

    pub fn pool_idle_timeout(mut self, timeout: Duration) -> Self {
        self.pool_idle_timeout = timeout;
        self
    }

    pub fn pool_max_idle_per_host(mut self, max: usize) -> Self {
        self.pool_max_idle_per_host = max;
        self
    }

    pub fn build(self) -> VestaResult<Client> {
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

pub fn create_api_client() -> VestaResult<Client> {
    HttpClientBuilder::new()
        .timeout(Duration::from_secs(60))
        .connect_timeout(Duration::from_secs(15))
        .accept_invalid_certs(false)
        .build()
}

pub fn create_ping_client() -> VestaResult<Client> {
    HttpClientBuilder::new()
        .timeout(Duration::from_secs(5))
        .connect_timeout(Duration::from_secs(3))
        .accept_invalid_certs(true)
        .pool_max_idle_per_host(5)
        .build()
}
