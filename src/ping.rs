use std::{sync::Arc, time::Duration};

use axum::{extract::Query, response::IntoResponse, Extension, Json};
use maud::{html, Markup};
use reqwest::{Client, Error, StatusCode};
use serde::Deserialize;
use serde_json::json;

use crate::{config::PingConfig, AppState};

#[derive(Deserialize)]
pub struct QueryParams {
    group: String,
    title: String,
}

async fn is_service_up(client: &reqwest::Client, ping_config: &PingConfig) -> Result<bool, Error> {
    let response = client
        .get(&ping_config.url)
        .timeout(Duration::new(5, 0))
        .send()
        .await?;
    Ok(response.status().is_success())
}

pub async fn ping_handler(
    Extension(state): Extension<Arc<AppState>>,
    Query(params): Query<QueryParams>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let config = &state.get_config();
    let service_info = config
        .get_service(&params.group, &params.title)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({"status": "fail", "message": "Service info not found"})),
            )
        })?;

    let ping_config = service_info.ping.as_ref().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "status": "error",
                "message": format!("Cannot get ping config of service '{}'", &service_info.title)
            })),
        )
    })?;

    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"status": "error", "message": "Failed to create HTTP client"})),
            )
        })?;

    let is_service_up = is_service_up(&client, ping_config).await.unwrap_or(false);

    if is_service_up {
        Ok(html!(
            div class="mr-4 w-2 h-2 bg-green-500 rounded-full" {}
        ))
    } else {
        Ok(html!(
            div class="mr-4 w-2 h-2 bg-red-500 rounded-full" {}
        ))
    }
}

pub fn render_service_indicator(group_id: &str, title: &str) -> Markup {
    html! {
        div
            class="w-2 h-2 visibility-hidden"
            hx-get=(format!("/api/ping?group={}&title={}", group_id, title))
            hx-trigger="load"
            hx-swap="outerHTML" { }

    }
}
