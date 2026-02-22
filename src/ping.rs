use std::{sync::Arc, time::Duration};

use axum::{Extension, extract::Query, response::IntoResponse};
use maud::{Markup, html};
use serde::Deserialize;

use crate::{
    AppState,
    config::PingConfig,
    error::{VestaError, VestaResult},
};

#[derive(Deserialize)]
pub struct QueryParams {
    group: String,
    title: String,
}

async fn is_service_up(client: &reqwest::Client, ping_config: &PingConfig) -> VestaResult<bool> {
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
) -> Result<impl IntoResponse, VestaError> {
    let config = state.get_config()?;
    let service_info = config
        .get_service(&params.group, &params.title)
        .ok_or_else(|| VestaError::ServiceNotFound {
            group: params.group.clone(),
            title: params.title.clone(),
        })?;

    let ping_config =
        service_info
            .ping
            .as_ref()
            .ok_or_else(|| VestaError::MissingWidgetConfig {
                service: service_info.title.clone(),
            })?;

    let client = state.get_http_client();
    let is_service_up = is_service_up(client, ping_config).await.unwrap_or(false);

    if is_service_up {
        Ok(html!(
            div class="w-2 h-2 bg-green-500 rounded-full" {}
        ))
    } else {
        Ok(html!(
            div class="w-2 h-2 bg-red-500 rounded-full" {}
        ))
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
