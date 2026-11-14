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
    html! {
        span class="service-ping" data-up=(is_up) aria-label=(if is_up { "Service online" } else { "Service unavailable" }) {
            span class="status-dot" aria-hidden="true" {}
        }
    }
}

/// Placeholder swapped by htmx for the real indicator on load.
pub fn service_indicator(group_id: &str, title: &str) -> Markup {
    let mut url = reqwest::Url::parse("http://vesta.invalid/api/ping")
        .expect("the static URL base must be valid");
    url.query_pairs_mut()
        .append_pair("group", group_id)
        .append_pair("title", title);
    html! {
        span
            class="service-ping"
            hx-get=(format!("{}?{}", url.path(), url.query().unwrap_or_default()))
            hx-trigger="load"
            hx-swap="outerHTML" aria-label="Checking service status" {
            span class="status-dot" aria-hidden="true" {}
        }
    }
}
