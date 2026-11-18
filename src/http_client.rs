use std::time::Duration;

use axum::http::StatusCode;
use reqwest::{Client, RequestBuilder};
use serde::de::DeserializeOwned;
use serde_json::Value;

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

/// Send an upstream HTTP request, map non-2xx statuses to `VestaError::ApiError`,
/// and decode the JSON body into `T`.
pub async fn send_checked_json<T, F>(request: RequestBuilder, error_message: F) -> VestaResult<T>
where
    T: DeserializeOwned,
    F: FnOnce(StatusCode) -> String + Send,
{
    let response = request.send().await?;
    let status = response.status();
    if !status.is_success() {
        return Err(VestaError::ApiError {
            status,
            message: error_message(status),
        });
    }
    Ok(response.json::<T>().await?)
}

/// Parse a JSON value into `T`, logging a truncated raw copy on shape
/// mismatch so upstream API drift is diagnosable from server logs.
pub fn parse_logged<T: DeserializeOwned>(
    widget_name: &str,
    endpoint: &str,
    value: &Value,
) -> VestaResult<T> {
    serde_json::from_value(value.clone()).map_err(|error| {
        let payload: String = value.to_string().chars().take(512).collect();
        tracing::warn!(
            widget = widget_name,
            endpoint,
            %error,
            payload,
            "{widget_name} payload did not match the expected shape"
        );
        VestaError::ApiError {
            status: StatusCode::BAD_GATEWAY,
            message: format!(
                "{widget_name} {endpoint} payload did not match the expected shape: {error}"
            ),
        }
    })
}

#[cfg(test)]
mod tests {
    use axum::{Router, routing::get};
    use reqwest::Client;

    use crate::error::VestaError;

    use super::send_checked_json;

    async fn unavailable() -> axum::http::StatusCode {
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    }

    #[tokio::test]
    async fn checked_json_preserves_the_callers_non_success_message() {
        let router = Router::new().route("/", get(unavailable));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });

        let error = send_checked_json::<serde_json::Value, _>(
            Client::new().get(format!("http://{address}/")),
            |status| {
                format!("AdGuard Home request to http://adguard/control/stats failed: {status}")
            },
        )
        .await
        .unwrap_err();
        server.abort();

        let VestaError::ApiError { status, message } = error else {
            panic!("expected an upstream status error");
        };
        assert_eq!(status, axum::http::StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            message,
            "AdGuard Home request to http://adguard/control/stats failed: 503 Service Unavailable"
        );
    }
}
