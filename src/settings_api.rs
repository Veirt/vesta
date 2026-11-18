use axum::Json;
use axum::body::Bytes;
use axum::extract::{Extension, Path, Query};
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::AppState;
use crate::config::{AppSettings, Dashboard, DashboardGroup, DashboardItem};
use crate::error::{VestaError, VestaResult};
use crate::metrics::node::NodeConfig;
use crate::response::jsend;
use crate::widget_system::WidgetQuery;

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

#[derive(Deserialize)]
pub struct ServiceQuery {
    pub id: String,
}

/// Body of `POST /api/widgets/{name}/options`: the edited widget's field key
/// and its unsaved options map.
#[derive(Deserialize)]
pub struct WidgetOptionsRequest {
    pub field: String,
    #[serde(default)]
    pub config: serde_json::Value,
}

/// Lists configured nodes: `GET /api/nodes`.
pub async fn list_nodes(
    Extension(state): Extension<std::sync::Arc<AppState>>,
) -> Result<impl IntoResponse, VestaError> {
    let config = state.config.snapshot();
    let nodes: Vec<_> = config
        .nodes
        .iter()
        .map(|n| json!({"id": n.id, "name": n.name, "endpoint": n.endpoint}))
        .collect();
    Ok(jsend::success(json!({ "nodes": nodes })))
}

/// Gets service configuration details: `GET /api/services`.
pub async fn get_service(
    Query(query): Query<ServiceQuery>,
    Extension(state): Extension<std::sync::Arc<AppState>>,
) -> Result<impl IntoResponse, VestaError> {
    let config = state.config.snapshot();
    let service = config
        .service(&query.id)
        .ok_or_else(|| VestaError::ServiceNotFound {
            id: query.id.clone(),
        })?;
    Ok(jsend::success(json!({ "service": service })))
}

/// Gets widget configuration details: `GET /api/widget`.
pub async fn get_widget(
    Query(query): Query<WidgetQuery>,
    Extension(state): Extension<std::sync::Arc<AppState>>,
) -> Result<impl IntoResponse, VestaError> {
    let widget = state
        .config
        .widget(&query.id)
        .ok_or_else(|| VestaError::WidgetNotFound {
            id: query.id.clone(),
        })?;
    Ok(jsend::success(json!({ "widget": widget })))
}

/// Resolves options for a widget's dynamic select field from unsaved form
/// values: `POST /api/widgets/{widget_name}/options`.
pub async fn widget_options(
    Path(widget_name): Path<String>,
    Extension(state): Extension<std::sync::Arc<AppState>>,
    Json(request): Json<WidgetOptionsRequest>,
) -> Result<impl IntoResponse, VestaError> {
    let options = state
        .widget_registry
        .current()
        .field_options(&widget_name, &request.field, &request.config)
        .await?;
    Ok(jsend::success(json!({ "options": options })))
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
    #[cfg(test)]
    state.lifecycle_test_hooks.dashboard_save_started().await;
    let lifecycle_guard = std::sync::Arc::clone(&state.plugin_ops).lock_owned().await;
    state
        .config
        .update_with_guard(lifecycle_guard, move |current| {
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
    use super::{DashboardItemsInput, sanitize_icon_name, save_dashboard_items, valid_icon_data};
    use serde_json::json;

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

    async fn test_app_state(
        prefix: &str,
    ) -> (
        crate::plugins::test_fixtures::TempDir,
        std::sync::Arc<crate::AppState>,
    ) {
        let sandbox = crate::plugins::test_fixtures::temp_dir(prefix);
        let static_dir = sandbox.path().join("static");
        std::fs::create_dir_all(&static_dir).unwrap();
        let settings = crate::Settings {
            config_path: sandbox.path().join("config.toml").display().to_string(),
            database_path: sandbox.path().join("vesta.db").display().to_string(),
            plugins_dir: sandbox.path().join("plugins").display().to_string(),
            bind_addr: "127.0.0.1:0".to_string(),
            static_dir: static_dir.display().to_string(),
            cookie_secure: false,
        };
        (sandbox, crate::AppState::new(&settings).await.unwrap())
    }

    fn counter_tile_input() -> DashboardItemsInput {
        serde_json::from_value(json!({
            "items": [{
                "kind": "widget",
                "id": "counter-1",
                "title": "Counter",
                "name": "Plugin:hello-world:counter",
                "config": { "label": "Hits", "start_value": 2 },
                "x": 0,
                "y": 0,
                "width": 3,
                "height": 2
            }]
        }))
        .unwrap()
    }

    #[tokio::test]
    async fn plugin_mutation_and_registry_publish_finish_after_request_cancellation() {
        let (_sandbox, state) = test_app_state("vesta-cancelled-plugin-lifecycle").await;
        let repo = crate::plugins::test_fixtures::git_fixture_repo(
            &crate::plugins::test_fixtures::fixture_dir("hello-world"),
            "cancelled-lifecycle",
        );
        let source = repo.path().display().to_string();
        let (mutation_finished, continue_publish) = state
            .lifecycle_test_hooks
            .hold_after_plugin_mutation()
            .await;

        let install = {
            let state = std::sync::Arc::clone(&state);
            tokio::spawn(async move { state.install_plugin(&source).await })
        };
        mutation_finished.await.unwrap();
        install.abort();
        assert!(install.await.unwrap_err().is_cancelled());
        continue_publish.send(()).unwrap();

        // Acquiring the shared gate is a barrier: the worker releases it only
        // after it has rebuilt and published the registry.
        let _lifecycle = std::sync::Arc::clone(&state.plugin_ops).lock_owned().await;
        assert!(state.plugins.record("hello-world").unwrap().is_some());
        assert!(
            state
                .widget_registry
                .current()
                .get("Plugin:hello-world:counter")
                .is_some()
        );
        state.nodes.shutdown();
    }

    #[tokio::test]
    async fn dashboard_save_serializes_with_uninstall_usage_check() {
        let (_sandbox, state) = test_app_state("vesta-plugin-dashboard-race").await;
        let repo = crate::plugins::test_fixtures::git_fixture_repo(
            &crate::plugins::test_fixtures::fixture_dir("hello-world"),
            "dashboard-race",
        );
        state
            .install_plugin(&repo.path().display().to_string())
            .await
            .unwrap();

        let (check_finished, continue_uninstall) = state
            .lifecycle_test_hooks
            .hold_after_uninstall_check()
            .await;
        let uninstall = {
            let state = std::sync::Arc::clone(&state);
            tokio::spawn(async move { state.uninstall_plugin("hello-world").await })
        };
        check_finished.await.unwrap();

        let save_attempted = state
            .lifecycle_test_hooks
            .watch_dashboard_save_attempt()
            .await;
        let save = {
            let state = std::sync::Arc::clone(&state);
            tokio::spawn(async move {
                save_dashboard_items(axum::Extension(state), axum::Json(counter_tile_input())).await
            })
        };
        save_attempted.await.unwrap();
        continue_uninstall.send(()).unwrap();

        uninstall.await.unwrap().unwrap();
        let save_result = save.await.unwrap();
        assert!(matches!(
            save_result,
            Err(crate::error::VestaError::InvalidWidgetConfig(_))
        ));
        assert!(state.config.snapshot().items.is_empty());
        assert!(state.plugins.record("hello-world").unwrap().is_none());
        assert!(
            state
                .widget_registry
                .current()
                .get("Plugin:hello-world:counter")
                .is_none()
        );
        state.nodes.shutdown();
    }

    #[tokio::test]
    async fn cancelled_dashboard_save_holds_lifecycle_guard_through_publication() {
        let (_sandbox, state) = test_app_state("vesta-cancelled-dashboard-save").await;
        let repo = crate::plugins::test_fixtures::git_fixture_repo(
            &crate::plugins::test_fixtures::fixture_dir("hello-world"),
            "cancelled-dashboard-save",
        );
        state
            .install_plugin(&repo.path().display().to_string())
            .await
            .unwrap();

        let (write_finished_tx, write_finished_rx) = tokio::sync::oneshot::channel();
        let (publish_tx, publish_rx) = std::sync::mpsc::channel();
        state.config.set_after_write_before_publish_hook(move || {
            write_finished_tx.send(()).unwrap();
            publish_rx.recv().unwrap();
        });

        let save = {
            let state = std::sync::Arc::clone(&state);
            tokio::spawn(async move {
                save_dashboard_items(axum::Extension(state), axum::Json(counter_tile_input())).await
            })
        };
        write_finished_rx.await.unwrap();
        save.abort();
        assert!(matches!(save.await, Err(error) if error.is_cancelled()));
        assert!(
            std::sync::Arc::clone(&state.plugin_ops)
                .try_lock_owned()
                .is_err()
        );
        assert!(state.config.snapshot().items.is_empty());

        publish_tx.send(()).unwrap();
        let uninstall_error = state.uninstall_plugin("hello-world").await.unwrap_err();
        assert!(matches!(
            uninstall_error,
            crate::error::VestaError::Plugin(message)
                if message.contains("still used by dashboard tiles")
        ));
        let snapshot = state.config.snapshot();
        assert_eq!(snapshot.items.len(), 1);
        assert!(state.plugins.record("hello-world").unwrap().is_some());
        state.nodes.shutdown();
    }
}
