use std::sync::Arc;

use axum::{Extension, extract::Query, response::IntoResponse};
use serde::Deserialize;
use serde_json::json;

use crate::services::ping_service::PingKey;
use crate::{AppState, error::VestaError, response::jsend};

#[derive(Deserialize)]
pub struct PingQuery {
    id: String,
}

pub async fn ping_handler(
    Extension(state): Extension<Arc<AppState>>,
    Query(params): Query<PingQuery>,
) -> Result<impl IntoResponse, VestaError> {
    let config = state.config.snapshot();
    let service = config
        .service(&params.id)
        .ok_or_else(|| VestaError::ServiceNotFound {
            id: params.id.clone(),
        })?;

    let ping_config = service.ping.as_ref().ok_or_else(|| {
        VestaError::InvalidDashboard(format!(
            "service '{}' has no availability check configured",
            service.id
        ))
    })?;

    let key = PingKey { id: params.id };
    let is_up = state.ping.check(&key, ping_config).await.unwrap_or(false);
    Ok(jsend::success(json!({ "up": is_up })))
}
