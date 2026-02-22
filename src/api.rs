use axum::{Extension, extract::Query, response::IntoResponse};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use crate::{AppState, error::VestaError, response::jsend};

#[derive(Deserialize)]
pub struct ServiceQuery {
    pub group: String,
    pub title: String,
}

#[derive(Deserialize)]
pub struct HealthQuery {
    #[serde(default)]
    pub detailed: bool,
}

/// Get service information
pub async fn get_service(
    Query(query): Query<ServiceQuery>,
    Extension(state): Extension<Arc<AppState>>,
) -> Result<impl IntoResponse, VestaError> {
    let service = state
        .get_config_manager()
        .get_service(&query.group, &query.title)?
        .ok_or_else(|| VestaError::ServiceNotFound {
            group: query.group.clone(),
            title: query.title.clone(),
        })?;

    Ok(jsend::success(json!({
        "service": service
    })))
}

/// Get widget information  
pub async fn get_widget(
    Query(query): Query<ServiceQuery>,
    Extension(state): Extension<Arc<AppState>>,
) -> Result<impl IntoResponse, VestaError> {
    let widget = state
        .get_config_manager()
        .get_widget(&query.group, &query.title)?
        .ok_or_else(|| VestaError::WidgetNotFound {
            group: query.group.clone(),
            title: query.title.clone(),
        })?;

    Ok(jsend::success(json!({
        "widget": widget
    })))
}

/// Get application health status
pub async fn health(
    Query(query): Query<HealthQuery>,
    Extension(state): Extension<Arc<AppState>>,
) -> Result<impl IntoResponse, VestaError> {
    if query.detailed {
        let stats = state.get_config_manager().get_config_stats()?;
        let registered_widgets = state.get_widget_registry().get_registered_widgets();

        Ok(jsend::success(json!({
            "status": "healthy",
            "version": env!("CARGO_PKG_VERSION"),
            "statistics": stats,
            "widgets": {
                "registered": registered_widgets,
                "count": registered_widgets.len()
            }
        })))
    } else {
        Ok(jsend::success(json!({
            "status": "healthy"
        })))
    }
}

/// List all services
pub async fn list_services(
    Extension(state): Extension<Arc<AppState>>,
) -> Result<impl IntoResponse, VestaError> {
    let config = state.get_config_manager().read_config()?;

    let mut services = Vec::new();
    for (group_id, group) in &config.groups {
        for service in &group.services {
            services.push(json!({
                "group": group_id,
                "group_name": group.name,
                "title": service.title,
                "href": service.href,
                "has_widget": service.widget.is_some(),
                "has_ping": service.ping.is_some(),
                "widget_type": service.widget.as_ref().map(|w| &w.name)
            }));
        }
    }

    Ok(jsend::success(json!({
        "services": services,
        "count": services.len()
    })))
}

/// Validate configuration
pub async fn validate_config(
    Extension(state): Extension<Arc<AppState>>,
) -> Result<impl IntoResponse, VestaError> {
    match state.get_config_manager().validate_config() {
        Ok(()) => Ok(jsend::success_message("Configuration is valid")),
        Err(e) => Err(e),
    }
}

/// Reload configuration
pub async fn reload_config(
    Extension(state): Extension<Arc<AppState>>,
) -> Result<impl IntoResponse, VestaError> {
    state.reload_config()?;
    Ok(jsend::success_message(
        "Configuration reloaded successfully",
    ))
}
