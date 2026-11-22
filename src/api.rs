use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use async_stream::stream;
use axum::{
    Extension,
    extract::{Path, Query},
    http::header::{CACHE_CONTROL, CONTENT_TYPE},
    http::{HeaderValue, StatusCode},
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
};
use serde_json::json;

use crate::{
    AppState, error::VestaError, error::VestaResult, metrics::service::NodeObservation,
    response::jsend, services::ping_service::PingKey, widget_system::WidgetQuery,
};

#[derive(serde::Deserialize)]
pub struct PingQuery {
    id: String,
}

#[derive(serde::Deserialize)]
pub struct HealthQuery {
    #[serde(default)]
    pub detailed: bool,
}

fn stats_payload(stats: &crate::metrics::node::NodeStats) -> serde_json::Value {
    stats.to_json(true)
}

fn safe_source_label(endpoint: &str) -> String {
    let Ok(url) = reqwest::Url::parse(endpoint) else {
        return "Remote node exporter".to_string();
    };
    if !matches!(url.scheme(), "http" | "https") {
        return "Remote node exporter".to_string();
    }
    let Some(host) = url.host_str() else {
        return "Remote node exporter".to_string();
    };
    match url.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_string(),
    }
}

async fn dashboard_payload(state: &AppState) -> serde_json::Value {
    let config = state.config.snapshot();
    let mut nodes = Vec::with_capacity(config.nodes.len());
    for node in &config.nodes {
        let (stats, scrape_failed) = match state.nodes.observation(&node.id).await {
            Some(NodeObservation::Stats(stats)) => (Some(stats), false),
            Some(NodeObservation::Failed) => (None, true),
            None => (None, false),
        };
        let source = if node.is_local() {
            "Local system metrics".to_string()
        } else {
            safe_source_label(&node.endpoint)
        };
        nodes.push(json!({
            "id": node.id,
            "name": node.name,
            "hostname": node.hostname,
            "device_name": node.device_name,
            "icon": node.icon,
            "source": source,
            "interval_secs": node.interval_secs,
            "scrape_failed": scrape_failed,
            "stats": stats.as_ref().map(stats_payload),
        }));
    }

    let items = dashboard_items_payload(state, &config);

    let app = state.config.settings_snapshot().app.clone();
    json!({ "nodes": nodes, "items": items, "groups": config.groups, "app": app })
}

fn dashboard_items_payload(
    state: &AppState,
    config: &crate::config::Dashboard,
) -> Vec<serde_json::Value> {
    config
        .items
        .iter()
        .map(|item| match item {
            crate::config::DashboardItem::Service(service) => json!({
                "kind": "service",
                "id": service.id,
                "title": service.title,
                "group": service.group,
                "icon": service.icon,
                "href": service.href,
                "show_destination": service.show_destination,
                "x": service.x,
                "y": service.y,
                "width": service.width,
                "height": service.height,
                "has_ping": service.ping.is_some(),
            }),
            crate::config::DashboardItem::Widget(widget) => json!({
                "kind": "widget",
                "id": widget.id,
                "title": widget.title,
                "group": widget.group,
                "name": widget.name,
                "x": widget.x,
                "y": widget.y,
                "width": widget.width,
                "height": widget.height,
                "refresh_interval_secs": state.widget_registry.current().refresh_interval_secs(widget),
            }),
        })
        .collect()
}

pub async fn dashboard(Extension(state): Extension<Arc<AppState>>) -> impl IntoResponse {
    jsend::success(dashboard_payload(&state).await)
}

pub async fn dashboard_services(Extension(state): Extension<Arc<AppState>>) -> impl IntoResponse {
    let config = state.config.snapshot();
    jsend::success(json!({ "items": dashboard_items_payload(&state, &config) }))
}

pub async fn node_events(Extension(state): Extension<Arc<AppState>>) -> impl IntoResponse {
    let mut updates = state.nodes.subscribe_updates();
    let mut config_updates = state.config.subscribe_updates();
    let initial = dashboard_payload(&state).await;
    let stream = stream! {
        yield Ok::<Event, Infallible>(Event::default().event("snapshot").data(initial.to_string()));

        loop {
            tokio::select! {
                update = updates.recv() => match update {
                    Ok((id, stats)) => {
                        let scrape_failed = stats.is_none();
                        let data = json!({
                            "id": id,
                            "stats": stats.as_ref().map(stats_payload),
                            "scrape_failed": scrape_failed,
                        });
                        yield Ok(Event::default().event("node").data(data.to_string()));
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        let snapshot = dashboard_payload(&state).await;
                        yield Ok(Event::default().event("snapshot").data(snapshot.to_string()));
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                },
                changed = config_updates.changed() => {
                    if changed.is_err() {
                        break;
                    }
                    let snapshot = dashboard_payload(&state).await;
                    yield Ok(Event::default().event("snapshot").data(snapshot.to_string()));
                }
            }
        }
    };

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}

pub async fn node_stats(
    Extension(state): Extension<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<impl IntoResponse, VestaError> {
    let stats = state.nodes.stats_for(&id).await?;
    let mut payload = stats_payload(&stats);
    if let Some(data) = payload.as_object_mut() {
        data.insert("node".to_string(), json!(id));
    }
    Ok(jsend::success(payload))
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

    let key = PingKey {
        id: params.id,
        url: ping_config.url.clone(),
    };
    let is_up = state.ping.check(&key, ping_config).await;
    Ok(jsend::success(json!({ "up": is_up })))
}

pub async fn widget_handler(
    Path(widget_name): Path<String>,
    Query(query): Query<WidgetQuery>,
    Extension(state): Extension<Arc<AppState>>,
) -> Result<impl IntoResponse, VestaError> {
    let widget = state
        .config
        .widget(&query.id)
        .ok_or_else(|| VestaError::WidgetNotFound {
            id: query.id.clone(),
        })?;
    let data = state
        .widget_registry
        .current()
        .handle_request(&widget_name, &widget, state.widget_ctx.clone(), query)
        .await?;
    Ok(jsend::success(data))
}

pub async fn plugin_asset(
    Extension(state): Extension<Arc<AppState>>,
    Path((plugin_id, path)): Path<(String, String)>,
) -> Result<Response, VestaError> {
    let root = state.plugins.plugin_dir(&plugin_id)?;
    let file = crate::plugins::assets::resolve_asset_path(&root, &path).ok_or(
        VestaError::PluginNotFound {
            id: format!("{}/{}", plugin_id, path),
        },
    )?;
    let content_type = crate::plugins::assets::content_type(&file)
        .ok_or(VestaError::PluginNotFound {
            id: format!("{}/{}", plugin_id, path),
        })?
        .to_string();
    let bytes = tokio::fs::read(&file).await.map_err(VestaError::Io)?;
    let headers = [
        (CONTENT_TYPE, content_type),
        (CACHE_CONTROL, "no-cache".to_string()),
        (
            axum::http::header::CONTENT_SECURITY_POLICY,
            "frame-ancestors 'self'".to_string(),
        ),
        (
            axum::http::header::X_CONTENT_TYPE_OPTIONS,
            "nosniff".to_string(),
        ),
    ];
    Ok((headers, bytes).into_response())
}

pub async fn get_icon(
    Extension(state): Extension<Arc<AppState>>,
    Path(icon_id): Path<i64>,
) -> VestaResult<Response> {
    let Some(icon) = state.database.load_icon(icon_id).await? else {
        return Err(VestaError::ApiError {
            status: StatusCode::NOT_FOUND,
            message: "Icon not found".to_string(),
        });
    };
    let mut response = (StatusCode::OK, icon.data).into_response();
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_str(&icon.content_type).map_err(|error| {
            VestaError::Config(format!("invalid stored icon content type: {error}"))
        })?,
    );
    response.headers_mut().insert(
        CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=300"),
    );
    response.headers_mut().insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    Ok(response)
}

pub async fn health(
    Query(query): Query<HealthQuery>,
    Extension(state): Extension<Arc<AppState>>,
) -> Result<impl IntoResponse, VestaError> {
    if !query.detailed {
        return Ok(jsend::success(json!({ "status": "healthy" })));
    }

    let config = state.config.snapshot();
    let total_services = config.services().count();
    let services_with_ping = config
        .services()
        .filter(|service| service.ping.is_some())
        .count();
    let widgets_in_use = config
        .widgets()
        .map(|widget| widget.name.clone())
        .collect::<Vec<_>>();

    Ok(jsend::success(json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
        "statistics": {
            "total_services": total_services,
            "total_widgets": widgets_in_use.len(),
            "services_with_ping": services_with_ping,
            "nodes": config.nodes.len(),
        },
        "widgets": {
            "registered": state.widget_registry.current().names(),
            "in_use": widgets_in_use,
        }
    })))
}

#[cfg(test)]
mod tests {
    use super::safe_source_label;

    #[test]
    fn remote_source_label_excludes_credentials_and_path() {
        assert_eq!(
            safe_source_label("https://user:pass@example.test:9100/metrics?token=private"),
            "example.test:9100"
        );
    }

    #[test]
    fn malformed_remote_source_uses_generic_label() {
        assert_eq!(safe_source_label("metrics.sock"), "Remote node exporter");
    }
}
