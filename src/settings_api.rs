use axum::Json;
use axum::extract::{Extension, Path, Query};
use axum::response::IntoResponse;
use serde::Deserialize;
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
pub struct ServiceQuery {
    pub id: String,
}

#[derive(Deserialize)]
pub struct WidgetOptionsRequest {
    pub field: String,
    #[serde(default)]
    pub config: serde_json::Value,
}

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
    let node_service = std::sync::Arc::clone(&state.nodes);
    state
        .config
        .update_with_post_publish(
            move |current| {
                Ok((
                    Dashboard {
                        nodes,
                        items: current.dashboard.items.clone(),
                        groups: current.dashboard.groups.clone(),
                    },
                    app,
                ))
            },
            move |snapshot| {
                node_service.reconfigure(&snapshot.dashboard.nodes);
                Ok(())
            },
        )
        .await?;
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

#[cfg(test)]
mod tests {
    use super::{
        DashboardInput, DashboardItemsInput, SettingsInput, save_dashboard_items, save_settings,
    };
    use crate::state::test_app_state;
    use serde_json::json;

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
        let (_sandbox, _settings, state) = test_app_state("vesta-cancelled-plugin-lifecycle").await;
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
        let (_sandbox, _settings, state) = test_app_state("vesta-plugin-dashboard-race").await;
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
        let (_sandbox, _settings, state) = test_app_state("vesta-cancelled-dashboard-save").await;
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

    #[tokio::test]
    async fn cancelled_settings_save_reconfigures_nodes_after_publication() {
        let (_sandbox, _settings, state) = test_app_state("vesta-cancelled-settings-save").await;
        let node = crate::metrics::node::NodeConfig {
            id: "settings-node".to_string(),
            name: "Settings Node".to_string(),
            hostname: None,
            device_name: None,
            icon: None,
            endpoint: "local".to_string(),
            interval_secs: 15,
        };
        let (write_finished_tx, write_finished_rx) = tokio::sync::oneshot::channel();
        let (publish_tx, publish_rx) = std::sync::mpsc::channel();
        state.config.set_after_write_before_publish_hook(move || {
            write_finished_tx.send(()).unwrap();
            publish_rx.recv().unwrap();
        });

        let save = {
            let state = std::sync::Arc::clone(&state);
            tokio::spawn(async move {
                save_settings(
                    axum::Extension(state),
                    axum::Json(SettingsInput {
                        dashboard: DashboardInput { nodes: vec![node] },
                        app: crate::config::AppSettings::default(),
                    }),
                )
                .await
            })
        };
        write_finished_rx.await.unwrap();
        assert!(state.nodes.node_ids().is_empty());
        save.abort();
        assert!(matches!(save.await, Err(error) if error.is_cancelled()));
        publish_tx.send(()).unwrap();

        state
            .config
            .update(|current| Ok(((*current.dashboard).clone(), current.app.clone())))
            .await
            .unwrap();

        assert_eq!(
            state
                .config
                .snapshot()
                .nodes
                .iter()
                .map(|node| node.id.as_str())
                .collect::<Vec<_>>(),
            ["settings-node"]
        );
        assert_eq!(state.nodes.node_ids(), ["settings-node"]);
        state.nodes.shutdown();
    }
}
