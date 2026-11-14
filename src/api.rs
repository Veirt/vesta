use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use async_stream::stream;
use axum::{
    Extension,
    extract::Query,
    response::{
        IntoResponse,
        sse::{Event, KeepAlive, Sse},
    },
};
use serde_json::json;

use crate::{AppState, error::VestaError, response::jsend};

#[derive(serde::Deserialize)]
pub struct ServiceQuery {
    pub group: String,
    pub title: String,
}

#[derive(serde::Deserialize)]
pub struct HealthQuery {
    #[serde(default)]
    pub detailed: bool,
}

fn stats_payload(stats: &crate::metrics::node::NodeStats) -> serde_json::Value {
    json!({
        "cpu_usage_percent": stats.cpu_usage_percent,
        "memory": {
            "used_bytes": stats.memory_used_bytes,
            "total_bytes": stats.memory_total_bytes,
            "usage_percent": stats.memory_usage_percent(),
        },
        "disk": {
            "mount": stats.disk_mount,
            "used_bytes": stats.disk_used_bytes,
            "total_bytes": stats.disk_total_bytes,
            "usage_percent": stats.disk_usage_percent(),
        },
        "load": stats.load.map(|load| json!({
            "one": load.one, "five": load.five, "fifteen": load.fifteen,
        })),
        "uptime_secs": stats.uptime_secs,
        "collected_age_secs": stats.collected_at.elapsed().as_secs(),
    })
}

fn safe_source_label(endpoint: &str) -> String {
    let Some(authority) = endpoint
        .strip_prefix("https://")
        .or_else(|| endpoint.strip_prefix("http://"))
        .and_then(|rest| rest.split(['/', '?', '#']).next())
    else {
        return "Remote node exporter".to_string();
    };

    let host = authority.rsplit('@').next().unwrap_or_default();
    if host.is_empty() {
        "Remote node exporter".to_string()
    } else {
        host.to_string()
    }
}

async fn dashboard_payload(state: &AppState) -> serde_json::Value {
    let config = state.config.snapshot();
    let mut nodes = Vec::with_capacity(config.nodes.len());
    for node in &config.nodes {
        let stats = state.nodes.stats_for(&node.id).await.ok();
        let scrape_failed = state.nodes.scrape_failed(&node.id).await;
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
            "stats": if scrape_failed {
                None
            } else {
                stats.as_ref().map(stats_payload)
            },
        }));
    }

    let groups = config
        .groups
        .iter()
        .map(|(id, group)| json!({ "id": id, "name": group.name }))
        .collect::<Vec<_>>();

    let app = state.config.settings_snapshot().app.clone();
    json!({ "nodes": nodes, "groups": groups, "app": app })
}

/// Initial dashboard state: `GET /api/dashboard`.
pub async fn dashboard(Extension(state): Extension<Arc<AppState>>) -> impl IntoResponse {
    jsend::success(dashboard_payload(&state).await)
}

/// Live node scrape updates: `GET /api/nodes/events`.
/// Each connection receives a full snapshot before incremental node events.
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

/// One node's current stats: `GET /api/nodes/{id}/stats`.
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

/// List configured nodes: `GET /api/nodes`.
pub async fn list_nodes(
    Extension(state): Extension<Arc<AppState>>,
) -> Result<impl IntoResponse, VestaError> {
    let config = state.config.snapshot();
    let nodes: Vec<_> = config
        .nodes
        .iter()
        .map(|n| json!({"id": n.id, "name": n.name, "endpoint": n.endpoint}))
        .collect();
    Ok(jsend::success(json!({ "nodes": nodes })))
}

/// Get service information.
pub async fn get_service(
    Query(query): Query<ServiceQuery>,
    Extension(state): Extension<Arc<AppState>>,
) -> Result<impl IntoResponse, VestaError> {
    let config = state.config.snapshot();
    let service =
        config
            .service(&query.group, &query.title)
            .ok_or_else(|| VestaError::ServiceNotFound {
                group: query.group.clone(),
                title: query.title.clone(),
            })?;
    Ok(jsend::success(json!({ "service": service })))
}

/// Get widget information.
pub async fn get_widget(
    Query(query): Query<ServiceQuery>,
    Extension(state): Extension<Arc<AppState>>,
) -> Result<impl IntoResponse, VestaError> {
    let widget = state
        .config
        .widget(&query.group, &query.title)?
        .ok_or_else(|| VestaError::WidgetNotFound {
            group: query.group.clone(),
            title: query.title.clone(),
        })?;
    Ok(jsend::success(json!({ "widget": widget })))
}

/// Application health; `?detailed=true` adds config statistics.
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
        .filter(|(_, _, s)| s.ping.is_some())
        .count();
    let widgets_by_group = config
        .services()
        .filter_map(|(_, _, s)| s.widget.as_ref().map(|w| w.name.clone()))
        .collect::<Vec<_>>();

    Ok(jsend::success(json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
        "statistics": {
            "total_groups": config.groups.len(),
            "total_services": total_services,
            "services_with_ping": services_with_ping,
            "nodes": config.nodes.len(),
        },
        "widgets": {
            "registered": state.widget_registry.names(),
            "in_use": widgets_by_group,
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
