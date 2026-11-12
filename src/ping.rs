use std::sync::Arc;

use axum::{Extension, extract::Query, response::IntoResponse};
use maud::{Markup, html};
use serde::Deserialize;

use crate::services::ping_service::PingKey;
use crate::{AppState, error::VestaError};

#[derive(Deserialize)]
pub struct PingQuery {
    group: String,
    title: String,
}

pub async fn ping_handler(
    Extension(state): Extension<Arc<AppState>>,
    Query(params): Query<PingQuery>,
) -> Result<impl IntoResponse, VestaError> {
    let config = state.config.snapshot();
    let service = config
        .service(&params.group, &params.title)
        .ok_or_else(|| VestaError::ServiceNotFound {
            group: params.group.clone(),
            title: params.title.clone(),
        })?;

    let ping_config = service
        .ping
        .as_ref()
        .ok_or_else(|| VestaError::MissingWidgetConfig {
            service: service.title.clone(),
        })?;

    let key = PingKey {
        group: params.group,
        title: params.title,
    };
    let is_up = state.ping.check(&key, ping_config).await.unwrap_or(false);
    Ok(indicator_markup(is_up))
}

fn indicator_markup(is_up: bool) -> Markup {
    let class = if is_up { "bg-green-500" } else { "bg-red-500" };
    html!(div class=(format!("w-2 h-2 {class} rounded-full")) {})
}

/// Placeholder swapped by htmx for the real indicator on load.
pub fn service_indicator(group_id: &str, title: &str) -> Markup {
    html! {
        div
            class="w-2 h-2 bg-slate-900"
            hx-get=(format!("/api/ping?group={}&title={}", group_id, title))
            hx-trigger="load"
            hx-swap="outerHTML" { }
    }
}
