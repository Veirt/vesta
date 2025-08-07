use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
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

    #[error("Missing credentials: {field}")]
    MissingCredentials { field: String },

    #[error("API error: {status} - {message}")]
    ApiError { status: StatusCode, message: String },

    #[error("Internal server error: {0}")]
    Internal(String),
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Failed to parse TOML: {0}")]
    TomlError(#[from] toml::de::Error),
}

impl IntoResponse for VestaError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            VestaError::Config(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Configuration error"),
            VestaError::Http(_) => (StatusCode::BAD_GATEWAY, "External service error"),
            VestaError::ServiceNotFound { .. } => (StatusCode::NOT_FOUND, "Service not found"),
            VestaError::WidgetNotFound { .. } => (StatusCode::NOT_FOUND, "Widget not found"),
            VestaError::MissingWidgetConfig { .. } => {
                (StatusCode::BAD_REQUEST, "Missing widget configuration")
            }
            VestaError::MissingCredentials { .. } => {
                (StatusCode::BAD_REQUEST, "Missing credentials")
            }
            VestaError::ApiError { status, .. } => (*status, "API error"),
            VestaError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error"),
        };

        let body = Json(json!({
            "status": "error",
            "message": error_message,
            "details": self.to_string(),
        }));

        (status, body).into_response()
    }
}

pub type VestaResult<T> = Result<T, VestaError>;
