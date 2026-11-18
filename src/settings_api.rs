use axum::Json;
use axum::body::Bytes;
use axum::extract::{Extension, Path, Query};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::AppState;
use crate::config::{AppSettings, Dashboard, DashboardGroup, DashboardItem};
use crate::error::{VestaError, VestaResult};
use crate::metrics::node::NodeConfig;
use crate::response::jsend;

#[derive(Deserialize)]
pub struct DashboardInput {
    #[serde(default)]
    nodes: Vec<NodeConfig>,
}

#[derive(Deserialize)]
pub struct SettingsInput {
    dashboard: DashboardInput,
    #[serde(default)]
    app: AppSettings,
}

#[derive(Deserialize)]
pub struct IconUploadQuery {
    name: String,
}

#[derive(Deserialize)]
pub struct PingTestInput {
    url: String,
}

#[derive(Deserialize)]
pub struct DashboardItemsInput {
    #[serde(default)]
    items: Vec<DashboardItem>,
    groups: Option<Vec<DashboardGroup>>,
}

#[derive(Deserialize)]
pub struct PluginInstallInput {
    url: String,
}

pub async fn get_settings(
    Extension(state): Extension<std::sync::Arc<AppState>>,
) -> Result<impl IntoResponse, VestaError> {
    let snapshot = state.config.settings_snapshot();
    Ok(jsend::success(json!({
        "dashboard": {
            "nodes": snapshot.dashboard.nodes,
            "items": snapshot.dashboard.items,
            "groups": snapshot.dashboard.groups,
        },
        "app": snapshot.app,
        "widgets": state.widget_registry.current().catalog_descriptors(),
    })))
}

pub async fn save_settings(
    Extension(state): Extension<std::sync::Arc<AppState>>,
    Json(input): Json<SettingsInput>,
) -> VestaResult<impl IntoResponse> {
    let nodes = input.dashboard.nodes;
    let app = input.app;
    let snapshot = state
        .config
        .update(|current| {
            Ok((
                Dashboard {
                    nodes,
                    items: current.dashboard.items.clone(),
                    groups: current.dashboard.groups.clone(),
                },
                app,
            ))
        })
        .await?;
    state.nodes.reconfigure(&snapshot.dashboard.nodes)?;
    Ok(jsend::success_message("Dashboard settings saved"))
}

pub async fn save_dashboard_items(
    Extension(state): Extension<std::sync::Arc<AppState>>,
    Json(input): Json<DashboardItemsInput>,
) -> VestaResult<impl IntoResponse> {
    state
        .config
        .update(|current| {
            Ok((
                Dashboard {
                    nodes: current.dashboard.nodes.clone(),
                    items: input.items,
                    groups: input
                        .groups
                        .unwrap_or_else(|| current.dashboard.groups.clone()),
                },
                current.app.clone(),
            ))
        })
        .await?;
    Ok(jsend::success_message("Dashboard layout saved"))
}

pub async fn test_ping_url(
    Extension(state): Extension<std::sync::Arc<AppState>>,
    Json(input): Json<PingTestInput>,
) -> VestaResult<impl IntoResponse> {
    if !crate::validation::valid_http_url(&input.url) {
        return Err(VestaError::InvalidDashboard(
            "availability check must be a valid http(s) URL".to_string(),
        ));
    }
    let up = state.ping.test_url(&input.url).await;
    Ok(jsend::success(json!({ "up": up })))
}

pub async fn upload_icon(
    Extension(state): Extension<std::sync::Arc<AppState>>,
    Query(query): Query<IconUploadQuery>,
    headers: HeaderMap,
    bytes: Bytes,
) -> VestaResult<impl IntoResponse> {
    if bytes.is_empty() || bytes.len() > 2 * 1024 * 1024 {
        return Err(VestaError::ApiError {
            status: StatusCode::PAYLOAD_TOO_LARGE,
            message: "Icon must be between 1 byte and 2 MiB".to_string(),
        });
    }
    let content_type = headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(str::trim)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !valid_icon_data(&content_type, &bytes) {
        return Err(VestaError::ApiError {
            status: StatusCode::UNSUPPORTED_MEDIA_TYPE,
            message: "Upload a valid PNG, JPEG, WebP, or GIF icon".to_string(),
        });
    }

    let name = sanitize_icon_name(&query.name);
    let id = state
        .database
        .store_icon(&name, &content_type, &bytes)
        .await?;
    Ok(jsend::success(json!({
        "id": id,
        "name": name,
        "content_type": content_type,
        "url": format!("/api/icons/{id}"),
    })))
}

pub async fn get_icon(
    Extension(state): Extension<std::sync::Arc<AppState>>,
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

fn valid_icon_data(content_type: &str, bytes: &[u8]) -> bool {
    match content_type {
        "image/png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "image/jpeg" => bytes.starts_with(&[0xff, 0xd8, 0xff]),
        "image/webp" => bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP",
        "image/gif" => bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a"),
        _ => false,
    }
}

fn sanitize_icon_name(name: &str) -> String {
    let basename = name.rsplit(['/', '\\']).next().unwrap_or("icon").trim();
    let cleaned: String = basename
        .chars()
        .filter(|character| !character.is_control())
        .take(120)
        .collect();
    if cleaned.is_empty() {
        "icon".to_string()
    } else {
        cleaned
    }
}

/// Lists every installed plugin with its manifest and provenance metadata.
pub async fn list_plugins(
    Extension(state): Extension<std::sync::Arc<AppState>>,
) -> Result<impl IntoResponse, VestaError> {
    let plugins = std::sync::Arc::clone(&state.plugins);
    let records = tokio::task::spawn_blocking(move || plugins.load_all())
        .await
        .map_err(|error| VestaError::Plugin(format!("plugin listing failed: {error}")))?;
    let plugins: Vec<PluginSummary> = records
        .into_iter()
        .map(PluginSummary::from_record)
        .collect();
    Ok(jsend::success(json!({ "plugins": plugins })))
}

/// Installs a plugin from a git URL and reloads the widget registry so its
/// widgets become available immediately. The manager rejects empty URLs;
/// the route is admin-gated.
pub async fn install_plugin(
    Extension(state): Extension<std::sync::Arc<AppState>>,
    Json(input): Json<PluginInstallInput>,
) -> Result<impl IntoResponse, VestaError> {
    let record = state.install_plugin(input.url.trim()).await?;
    Ok(jsend::success(PluginSummary::from_record(record)))
}

/// Re-fetches an installed plugin from its recorded source URL and swaps the
/// tree in place, then reloads the widget registry.
pub async fn update_plugin(
    Extension(state): Extension<std::sync::Arc<AppState>>,
    Path(plugin_id): Path<String>,
) -> Result<impl IntoResponse, VestaError> {
    let record = state.update_plugin(&plugin_id).await?;
    Ok(jsend::success(PluginSummary::from_record(record)))
}

/// Removes an installed plugin. Refuses while dashboard tiles still use it.
pub async fn uninstall_plugin(
    Extension(state): Extension<std::sync::Arc<AppState>>,
    Path(plugin_id): Path<String>,
) -> Result<impl IntoResponse, VestaError> {
    state.uninstall_plugin(&plugin_id).await?;
    Ok(jsend::success(json!({ "removed": plugin_id })))
}

/// Serves a plugin's static widget assets. Public like the dashboard itself;
/// path resolution rejects traversal and symlink escapes.
pub async fn plugin_asset(
    Extension(state): Extension<std::sync::Arc<AppState>>,
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

impl PluginSummary {
    fn from_record(record: crate::plugins::manifest::PluginRecord) -> Self {
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
                .map(|widget| PluginWidgetSummary {
                    id: widget.id,
                    title: widget.title,
                    description: widget.description,
                    entry: widget.entry,
                    width: widget.width,
                    height: widget.height,
                    refresh_interval_secs: widget.refresh_interval_secs,
                    fields: widget
                        .fields
                        .into_iter()
                        .map(|field| PluginFieldSummary {
                            key: field.key,
                            label: field.label,
                            kind: field.kind,
                            required: field.required,
                            description: field.description,
                            default: field.default,
                            options: field
                                .options
                                .into_iter()
                                .map(|option| PluginOptionSummary {
                                    value: option.value,
                                    label: option.label,
                                })
                                .collect(),
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{sanitize_icon_name, valid_icon_data};

    #[test]
    fn uploaded_icons_must_match_a_supported_raster_signature() {
        assert!(valid_icon_data("image/png", b"\x89PNG\r\n\x1a\nrest"));
        assert!(valid_icon_data("image/webp", b"RIFF0000WEBPrest"));
        assert!(!valid_icon_data("image/png", b"<svg></svg>"));
        assert!(!valid_icon_data("image/svg+xml", b"<svg></svg>"));
    }

    #[test]
    fn icon_filename_cannot_include_a_path() {
        assert_eq!(sanitize_icon_name("../../jellyfin.png"), "jellyfin.png");
        assert_eq!(sanitize_icon_name("\n"), "icon");
    }
}
