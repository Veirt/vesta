use std::sync::Arc;

use axum::{Extension, extract::Query, response::IntoResponse};
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

/// One node's current stats: `GET /api/nodes/{id}/stats`.
pub async fn node_stats(
    Extension(state): Extension<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<impl IntoResponse, VestaError> {
    let stats = state.nodes.stats_for(&id).await?;
    Ok(jsend::success(json!({
        "node": id,
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
        "load": stats.load.map(|l| json!({
            "one": l.one, "five": l.five, "fifteen": l.fifteen,
        })),
        "uptime_secs": stats.uptime_secs,
    })))
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

/// Validate configuration without swapping it: `GET /api/config/validate`.
pub async fn validate_config(
    Extension(state): Extension<Arc<AppState>>,
) -> Result<impl IntoResponse, VestaError> {
    let config = state.config.snapshot();
    state.widget_registry.validate_dashboard(&config)?;
    crate::metrics::service::validate_nodes(&config.nodes).map_err(VestaError::Metrics)?;
    Ok(jsend::success_message("Configuration is valid"))
}

/// Reload configuration from disk: `POST /api/config/reload`.
pub async fn reload_config(
    Extension(state): Extension<Arc<AppState>>,
) -> Result<impl IntoResponse, VestaError> {
    state.config.reload()?;
    Ok(jsend::success_message(
        "Configuration reloaded successfully",
    ))
}
