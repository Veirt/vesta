use std::sync::Arc;

use axum::{Extension, extract::Query, response::IntoResponse};
use maud::{Markup, html};
use serde::Deserialize;

use crate::{AppState, error::VestaError};

#[derive(Deserialize)]
pub struct QueryParams {
    group: String,
    title: String,
}

pub async fn ping_handler(
    Extension(state): Extension<Arc<AppState>>,
    Query(params): Query<QueryParams>,
) -> Result<impl IntoResponse, VestaError> {
    let config = state.get_config()?;
    let service_info = config
        .get_service(&params.group, &params.title)
        .ok_or_else(|| VestaError::ServiceNotFound {
            group: params.group.clone(),
            title: params.title.clone(),
        })?;

    let ping_config = service_info
        .ping
        .as_ref()
        .ok_or_else(|| VestaError::MissingWidgetConfig {
            service: service_info.title.clone(),
        })?;

    let ping_service = state.get_ping_service();
    let is_up = ping_service
        .check_service(&params.group, &params.title, ping_config)
        .await
        .unwrap_or(false);

    Ok(render_ping_indicator(is_up))
}

fn render_ping_indicator(is_up: bool) -> Markup {
    if is_up {
        html!(div class="w-2 h-2 bg-green-500 rounded-full" {})
    } else {
        html!(div class="w-2 h-2 bg-red-500 rounded-full" {})
    }
}

pub fn render_service_indicator(group_id: &str, title: &str) -> Markup {
    html! {
        div
            class="w-2 h-2 bg-slate-900"
            hx-get=(format!("/api/ping?group={}&title={}", group_id, title))
            hx-trigger="load"
            hx-swap="outerHTML" { }
    }
}
