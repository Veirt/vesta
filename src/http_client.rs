use reqwest::Client;
use std::time::Duration;

use crate::error::{VestaError, VestaResult};

struct HttpClientSettings {
    timeout: Duration,
    connect_timeout: Duration,
    accept_invalid_certs: bool,
    pool_idle_timeout: Duration,
    pool_max_idle_per_host: usize,
}

impl Default for HttpClientSettings {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(10),
            // Home-server endpoints regularly use self-signed certificates.
            accept_invalid_certs: true,
            pool_idle_timeout: Duration::from_secs(30),
            pool_max_idle_per_host: 10,
        }
    }
}

pub fn create_default_client() -> VestaResult<Client> {
    let settings = HttpClientSettings::default();
    let mut builder = Client::builder()
        .timeout(settings.timeout)
        .connect_timeout(settings.connect_timeout)
        .pool_idle_timeout(settings.pool_idle_timeout)
        .pool_max_idle_per_host(settings.pool_max_idle_per_host);

    if settings.accept_invalid_certs {
        builder = builder.danger_accept_invalid_certs(true);
    }

    builder
        .build()
        .map_err(|e| VestaError::Config(format!("Failed to create HTTP client: {e}")))
}
