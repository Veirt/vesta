use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

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

    pub fn fail(data: T) -> Self {
        Self::Fail { data }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::Error {
            message: message.into(),
            code: None,
            data: None,
        }
    }

    pub fn error_with_code(message: impl Into<String>, code: u32) -> Self {
        Self::Error {
            message: message.into(),
            code: Some(code),
            data: None,
        }
    }

    pub fn error_with_data(message: impl Into<String>, data: T) -> Self {
        Self::Error {
            message: message.into(),
            code: None,
            data: Some(data),
        }
    }

    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }

    pub fn is_fail(&self) -> bool {
        matches!(self, Self::Fail { .. })
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error { .. })
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

    pub fn success_empty() -> JSendResponse<Value> {
        JSendResponse::success(json!({}))
    }

    pub fn success_message(message: impl Into<String>) -> JSendResponse<Value> {
        JSendResponse::success(json!({ "message": message.into() }))
    }

    pub fn fail<T: Serialize>(data: T) -> JSendResponse<T> {
        JSendResponse::fail(data)
    }

    pub fn fail_message(message: impl Into<String>) -> JSendResponse<Value> {
        JSendResponse::fail(json!({ "message": message.into() }))
    }

    pub fn fail_validation(
        field: impl Into<String>,
        message: impl Into<String>,
    ) -> JSendResponse<Value> {
        JSendResponse::fail(json!({
            "validation": {
                field.into(): message.into()
            }
        }))
    }

    pub fn error(message: impl Into<String>) -> JSendResponse<Value> {
        JSendResponse::error(message)
    }

    pub fn error_not_found(resource: impl Into<String>) -> JSendResponse<Value> {
        JSendResponse::Error {
            message: format!("{} not found", resource.into()),
            code: Some(404),
            data: None,
        }
    }

    pub fn error_unauthorized() -> JSendResponse<Value> {
        JSendResponse::Error {
            message: "Unauthorized".to_string(),
            code: Some(401),
            data: None,
        }
    }

    pub fn error_forbidden() -> JSendResponse<Value> {
        JSendResponse::Error {
            message: "Forbidden".to_string(),
            code: Some(403),
            data: None,
        }
    }

    pub fn error_internal(message: impl Into<String>) -> JSendResponse<Value> {
        JSendResponse::Error {
            message: message.into(),
            code: Some(500),
            data: None,
        }
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
