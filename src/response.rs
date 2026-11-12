use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// JSend specification for API responses: https://github.com/omniti-labs/jsend
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
        let status = match &self {
            JSendResponse::Success { .. } => StatusCode::OK,
            JSendResponse::Fail { .. } => StatusCode::BAD_REQUEST,
            JSendResponse::Error { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(self)).into_response()
    }
}

pub mod jsend {
    use super::*;

    pub fn success<T: Serialize>(data: T) -> JSendResponse<T> {
        JSendResponse::success(data)
    }

    pub fn success_message(message: impl Into<String>) -> JSendResponse<Value> {
        JSendResponse::success(json!({ "message": message.into() }))
    }
}
