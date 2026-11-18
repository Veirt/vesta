use reqwest::Client;
use std::time::Duration;

use crate::error::{VestaError, VestaResult};

pub fn create_default_client() -> VestaResult<Client> {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .pool_idle_timeout(Duration::from_secs(30))
        .pool_max_idle_per_host(10)
        // Home-server endpoints regularly use self-signed certificates.
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| VestaError::Config(format!("Failed to create HTTP client: {e}")))
}
