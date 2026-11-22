use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Path};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::AppState;
use crate::error::VestaError;
use crate::plugins::manifest::{PluginFieldSpec, PluginOptionSpec, PluginRecord, PluginWidgetSpec};
use crate::response::jsend;

#[derive(Deserialize)]
pub struct PluginInstallInput {
    url: String,
}

pub async fn list_plugins(
    Extension(state): Extension<Arc<AppState>>,
) -> Result<impl IntoResponse, VestaError> {
    let plugins = Arc::clone(&state.plugins);
    let records = tokio::task::spawn_blocking(move || plugins.load_all())
        .await
        .map_err(|error| VestaError::Plugin(format!("plugin listing failed: {error}")))?;
    let plugins: Vec<PluginSummary> = records.into_iter().map(PluginSummary::from).collect();
    Ok(jsend::success(json!({ "plugins": plugins })))
}

pub async fn install_plugin(
    Extension(state): Extension<Arc<AppState>>,
    Json(input): Json<PluginInstallInput>,
) -> Result<impl IntoResponse, VestaError> {
    let record = state.install_plugin(input.url.trim()).await?;
    Ok(jsend::success(PluginSummary::from(record)))
}

pub async fn update_plugin(
    Extension(state): Extension<Arc<AppState>>,
    Path(plugin_id): Path<String>,
) -> Result<impl IntoResponse, VestaError> {
    let record = state.update_plugin(&plugin_id).await?;
    Ok(jsend::success(PluginSummary::from(record)))
}

pub async fn uninstall_plugin(
    Extension(state): Extension<Arc<AppState>>,
    Path(plugin_id): Path<String>,
) -> Result<impl IntoResponse, VestaError> {
    state.uninstall_plugin(&plugin_id).await?;
    Ok(jsend::success(json!({ "removed": plugin_id })))
}

#[derive(Serialize)]
struct PluginSummary {
    id: String,
    name: String,
    version: Option<String>,
    description: Option<String>,
    source_url: String,
    commit: String,
    installed_at: String,
    widgets: Vec<PluginWidgetSummary>,
}

#[derive(Serialize)]
struct PluginWidgetSummary {
    id: String,
    title: String,
    description: Option<String>,
    entry: String,
    width: u8,
    height: u8,
    refresh_interval_secs: Option<u64>,
    fields: Vec<PluginFieldSummary>,
}

#[derive(Serialize)]
struct PluginFieldSummary {
    key: String,
    label: String,
    kind: String,
    required: bool,
    description: Option<String>,
    default: Option<serde_json::Value>,
    options: Vec<PluginOptionSummary>,
}

#[derive(Serialize)]
struct PluginOptionSummary {
    value: String,
    label: String,
}

impl From<PluginOptionSpec> for PluginOptionSummary {
    fn from(option: PluginOptionSpec) -> Self {
        Self {
            value: option.value,
            label: option.label,
        }
    }
}

impl From<PluginFieldSpec> for PluginFieldSummary {
    fn from(field: PluginFieldSpec) -> Self {
        Self {
            key: field.key,
            label: field.label,
            kind: field.kind,
            required: field.required,
            description: field.description,
            default: field.default,
            options: field.options.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<PluginWidgetSpec> for PluginWidgetSummary {
    fn from(widget: PluginWidgetSpec) -> Self {
        Self {
            id: widget.id,
            title: widget.title,
            description: widget.description,
            entry: widget.entry,
            width: widget.width,
            height: widget.height,
            refresh_interval_secs: widget.refresh_interval_secs,
            fields: widget.fields.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<PluginRecord> for PluginSummary {
    fn from(record: PluginRecord) -> Self {
        Self {
            id: record.manifest.id,
            name: record.manifest.name,
            version: record.manifest.version,
            description: record.manifest.description,
            source_url: record.source.source_url,
            commit: record.source.commit,
            installed_at: record.source.installed_at,
            widgets: record
                .manifest
                .widgets
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}
