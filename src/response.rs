use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize)]
pub struct SuccessResponse<T = Value> {
    pub status: &'static str,
    pub data: T,
}

impl<T: Serialize> IntoResponse for SuccessResponse<T> {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

pub mod jsend {
    use super::*;

    pub fn success<T: Serialize>(data: T) -> SuccessResponse<T> {
        SuccessResponse {
            status: "success",
            data,
        }
    }

    pub fn success_message(message: impl Into<String>) -> SuccessResponse<Value> {
        success(json!({ "message": message.into() }))
    }
}
