use std::sync::Arc;

use axum::Router;
use axum::extract::{DefaultBodyLimit, Extension};
use axum::middleware;
use axum::routing::{get, post, put};
use tower_http::compression::CompressionLayer;
use tower_http::services::ServeDir;

use crate::state::AppState;
use crate::templates::dashboard;
use crate::{api, auth, icons_api, plugins_api, settings_api};

pub fn build_router(state: Arc<AppState>, static_dir: &str) -> Router {
    let settings_routes = Router::new()
        .route(
            "/api/settings",
            get(settings_api::get_settings).put(settings_api::save_settings),
        )
        .route("/api/settings/icons", post(icons_api::upload_icon))
        .route(
            "/api/settings/dashboard",
            put(settings_api::save_dashboard_items),
        )
        .route("/api/settings/ping-test", post(settings_api::test_ping_url))
        .route(
            "/api/plugins",
            get(plugins_api::list_plugins).post(plugins_api::install_plugin),
        )
        .route(
            "/api/plugins/{plugin_id}",
            put(plugins_api::update_plugin).delete(plugins_api::uninstall_plugin),
        )
        .route("/api/nodes", get(settings_api::list_nodes))
        .route("/api/services", get(settings_api::get_service))
        .route("/api/widget", get(settings_api::get_widget))
        .route(
            "/api/widgets/{widget_name}/options",
            post(settings_api::widget_options),
        )
        .route_layer(middleware::from_fn(auth::require_admin));

    Router::new()
        .route("/api/widgets/{widget_name}", get(api::widget_handler))
        .route("/api/ping", get(api::ping_handler))
        .route("/api/dashboard", get(api::dashboard))
        .route("/api/dashboard/services", get(api::dashboard_services))
        .route("/api/nodes/events", get(api::node_events))
        .route("/api/nodes/{node_id}/stats", get(api::node_stats))
        .route("/api/health", get(api::health))
        .route("/api/auth/status", get(auth::status))
        .route("/api/auth/setup", post(auth::setup))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/logout", post(auth::logout))
        .route("/api/icons/{icon_id}", get(api::get_icon))
        .merge(settings_routes)
        .route("/", get(dashboard))
        .route("/settings", get(dashboard))
        .route(
            "/api/plugins/{plugin_id}/assets/{*path}",
            get(api::plugin_asset),
        )
        .nest_service("/static", ServeDir::new(static_dir))
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
        .layer(CompressionLayer::new())
        .layer(Extension(state))
}
