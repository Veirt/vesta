use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum VestaError {
    #[error("Application configuration error: {0}")]
    Config(String),

    #[error("Dashboard settings are invalid: {0}")]
    InvalidDashboard(String),

    #[error("Database operation failed: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Database migration failed: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("Filesystem operation failed: {0}")]
    Io(#[from] std::io::Error),

    #[error("Data encoding failed: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Service not found: id='{id}'")]
    ServiceNotFound { id: String },

    #[error("Widget not found: id='{id}'")]
    WidgetNotFound { id: String },

    #[error("Invalid widget config: {0}")]
    InvalidWidgetConfig(String),

    #[error("Plugin error: {0}")]
    Plugin(String),

    #[error("Plugin not found: id='{id}'")]
    PluginNotFound { id: String },

    #[error("Metrics error: {0}")]
    Metrics(#[from] MetricsError),

    #[error("API error: {status} - {message}")]
    ApiError { status: StatusCode, message: String },
}

#[derive(Error, Debug)]
#[error("{0}")]
pub struct MetricsError(pub String);

pub type VestaResult<T> = Result<T, VestaError>;

impl IntoResponse for VestaError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            VestaError::Config(_) => (StatusCode::INTERNAL_SERVER_ERROR, "config_error"),
            VestaError::InvalidDashboard(_) => (StatusCode::BAD_REQUEST, "invalid_dashboard"),
            VestaError::Database(_) | VestaError::Migration(_) | VestaError::Io(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "storage_error")
            }
            VestaError::Serialization(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "serialization_error")
            }
            VestaError::Http(_) => (StatusCode::BAD_GATEWAY, "external_service_error"),
            VestaError::ServiceNotFound { .. } => (StatusCode::NOT_FOUND, "service_not_found"),
            VestaError::WidgetNotFound { .. } => (StatusCode::NOT_FOUND, "widget_not_found"),
            VestaError::InvalidWidgetConfig(_) => {
                (StatusCode::BAD_REQUEST, "invalid_widget_config")
            }
            VestaError::Plugin(_) => (StatusCode::BAD_REQUEST, "plugin_error"),
            VestaError::PluginNotFound { .. } => (StatusCode::NOT_FOUND, "plugin_not_found"),
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
