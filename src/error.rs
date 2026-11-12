use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum VestaError {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Service not found: group='{group}', title='{title}'")]
    ServiceNotFound { group: String, title: String },

    #[error("Widget not found: group='{group}', title='{title}'")]
    WidgetNotFound { group: String, title: String },

    #[error("Widget configuration missing for service '{service}'")]
    MissingWidgetConfig { service: String },

    #[error("Invalid widget config: {0}")]
    InvalidWidgetConfig(String),

    #[error("Metrics error: {0}")]
    Metrics(#[from] MetricsError),

    #[error("API error: {status} - {message}")]
    ApiError { status: StatusCode, message: String },
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Failed to parse TOML: {0}")]
    TomlError(#[from] toml::de::Error),
    #[error("Invalid configuration: {0}")]
    Invalid(String),
}

#[derive(Error, Debug)]
#[error("{0}")]
pub struct MetricsError(pub String);

pub type VestaResult<T> = Result<T, VestaError>;

impl IntoResponse for VestaError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            VestaError::Config(_) => (StatusCode::INTERNAL_SERVER_ERROR, "config_error"),
            VestaError::Http(_) => (StatusCode::BAD_GATEWAY, "external_service_error"),
            VestaError::ServiceNotFound { .. } => (StatusCode::NOT_FOUND, "service_not_found"),
            VestaError::WidgetNotFound { .. } => (StatusCode::NOT_FOUND, "widget_not_found"),
            VestaError::MissingWidgetConfig { .. } => {
                (StatusCode::BAD_REQUEST, "missing_widget_config")
            }
            VestaError::InvalidWidgetConfig(_) => {
                (StatusCode::BAD_REQUEST, "invalid_widget_config")
            }
            VestaError::Metrics(_) => (StatusCode::BAD_GATEWAY, "metrics_error"),
            VestaError::ApiError { status, .. } => (*status, "api_error"),
        };

        (
            status,
            Json(json!({
                "status": "error",
                "error": code,
                "message": self.to_string(),
            })),
        )
            .into_response()
    }
}
