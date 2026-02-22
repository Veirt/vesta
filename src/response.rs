use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// JSend specification implementation for API responses
/// See: https://github.com/omniti-labs/jsend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum JSendResponse<T = Value> {
    #[serde(rename = "success")]
    Success { data: T },

    #[serde(rename = "fail")]
    Fail { data: T },

    #[serde(rename = "error")]
    Error {
        message: String,
        code: Option<u32>,
        data: Option<T>,
    },
}

impl<T: Serialize> JSendResponse<T> {
    pub fn success(data: T) -> Self {
        Self::Success { data }
    }
}

impl<T: Serialize> IntoResponse for JSendResponse<T> {
    fn into_response(self) -> Response {
        let status_code = match &self {
            JSendResponse::Success { .. } => StatusCode::OK,
            JSendResponse::Fail { .. } => StatusCode::BAD_REQUEST,
            JSendResponse::Error { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status_code, Json(self)).into_response()
    }
}

/// Convenience functions for common JSend responses
pub mod jsend {
    use super::*;

    pub fn success<T: Serialize>(data: T) -> JSendResponse<T> {
        JSendResponse::success(data)
    }

    pub fn success_message(message: impl Into<String>) -> JSendResponse<Value> {
        JSendResponse::success(json!({ "message": message.into() }))
    }
}

/// Helper trait to convert VestaError to JSend responses
impl From<crate::error::VestaError> for JSendResponse<Value> {
    fn from(error: crate::error::VestaError) -> Self {
        match error {
            crate::error::VestaError::ServiceNotFound { group, title } => JSendResponse::Error {
                message: format!("Service not found: group='{}', title='{}'", group, title),
                code: Some(404),
                data: Some(json!({ "group": group, "title": title })),
            },
            crate::error::VestaError::WidgetNotFound { group, title } => JSendResponse::Error {
                message: format!("Widget not found: group='{}', title='{}'", group, title),
                code: Some(404),
                data: Some(json!({ "group": group, "title": title })),
            },
            crate::error::VestaError::MissingCredentials { field } => JSendResponse::Fail {
                data: json!({
                    "validation": {
                        field: "Missing required field"
                    }
                }),
            },
            crate::error::VestaError::MissingWidgetConfig { service } => JSendResponse::Fail {
                data: json!({
                    "error": "missing_widget_config",
                    "message": format!("Missing widget configuration for service: {}", service)
                }),
            },
            crate::error::VestaError::Config(config_error) => JSendResponse::Error {
                message: "Configuration error".to_string(),
                code: Some(500),
                data: Some(json!({ "details": config_error.to_string() })),
            },
            crate::error::VestaError::Http(http_error) => JSendResponse::Error {
                message: "External service error".to_string(),
                code: Some(502),
                data: Some(json!({ "details": http_error.to_string() })),
            },
            crate::error::VestaError::ApiError { status, message } => JSendResponse::Error {
                message: format!("API error: {}", message),
                code: Some(status.as_u16() as u32),
                data: None,
            },
            crate::error::VestaError::Internal(message) => JSendResponse::Error {
                message,
                code: Some(500),
                data: None,
            },
        }
    }
}
