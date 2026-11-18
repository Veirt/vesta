mod api;
mod auth;
mod config;
mod config_codec;
mod config_layout;
mod config_manager;
mod database;
mod error;
mod http_client;
mod metrics;
mod plugins;
mod response;
mod services;
mod settings_api;
mod templates;
mod validation;
mod widget_config;
mod widget_system;
mod widgets;

use std::sync::Arc;

use axum::Router;
use axum::extract::{DefaultBodyLimit, Extension};
use axum::middleware;
use axum::routing::{get, post, put};
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio::task::JoinHandle;
use tower_http::compression::CompressionLayer;
use tower_http::services::ServeDir;
use tracing_subscriber::EnvFilter;

use config_manager::ConfigManager;
use database::Database;
use error::{VestaError, VestaResult};
use http_client::create_default_client;
use metrics::service::NodeService;
use services::ping_service::PingService;
use templates::dashboard;
use widget_system::{SharedWidgetRegistry, WidgetContext, WidgetHandlerDyn, WidgetRegistry};
use widgets::adguard_home_widget::AdGuardHomeWidget;
use widgets::clock_widget::ClockWidget;
use widgets::jellyfin_random_show_widget::JellyfinRandomShowWidget;
use widgets::prowlarr_widget::ProwlarrWidget;
use widgets::quick_links_widget::QuickLinksWidget;
use widgets::sonarr_calendar_widget::SonarrCalendarWidget;
use widgets::system_stats_widget::SystemStatsWidget;
use widgets::weather_widget::WeatherWidget;

/// Runtime paths and listener settings. Dashboard configuration is stored in TOML.
struct Settings {
    config_path: String,
    database_path: String,
    plugins_dir: String,
    bind_addr: String,
    static_dir: String,
    cookie_secure: bool,
}

impl Settings {
    fn from_env() -> Self {
        Self {
            config_path: env_or("VESTA_CONFIG", "./config.toml"),
            database_path: env_or("VESTA_DATABASE", "./data/vesta.sqlite"),
            plugins_dir: env_or("VESTA_PLUGINS", "./data/plugins"),
            bind_addr: env_or("VESTA_BIND", "0.0.0.0:3000"),
            static_dir: env_or("VESTA_STATIC_DIR", "static"),
            cookie_secure: env_or("VESTA_COOKIE_SECURE", "false") == "true",
        }
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

pub struct AppState {
    pub(crate) config: Arc<ConfigManager>,
    pub(crate) database: Arc<Database>,
    pub(crate) nodes: Arc<NodeService>,
    pub(crate) ping: PingService,
    pub(crate) widget_registry: SharedWidgetRegistry,
    pub(crate) plugins: Arc<plugins::manager::PluginManager>,
    /// Shared transaction gate for plugin lifecycle work and dashboard saves.
    pub(crate) plugin_ops: Arc<Mutex<()>>,
    plugin_requests: mpsc::Sender<PluginRequest>,
    _plugin_lifecycle_worker: JoinHandle<()>,
    #[cfg(test)]
    pub(crate) lifecycle_test_hooks: Arc<LifecycleTestHooks>,
    pub(crate) widget_ctx: WidgetContext,
    pub(crate) cookie_secure: bool,
}

const PLUGIN_LIFECYCLE_QUEUE_CAPACITY: usize = 8;

enum PluginRequest {
    Install {
        url: String,
        reply: oneshot::Sender<VestaResult<plugins::manifest::PluginRecord>>,
    },
    Update {
        plugin_id: String,
        reply: oneshot::Sender<VestaResult<plugins::manifest::PluginRecord>>,
    },
    Uninstall {
        plugin_id: String,
        reply: oneshot::Sender<VestaResult<()>>,
    },
}

#[cfg(test)]
struct LifecycleTestPause {
    entered: oneshot::Sender<()>,
    release: oneshot::Receiver<()>,
}

#[cfg(test)]
#[derive(Default)]
pub(crate) struct LifecycleTestHooks {
    after_plugin_mutation: Mutex<Option<LifecycleTestPause>>,
    after_uninstall_check: Mutex<Option<LifecycleTestPause>>,
    dashboard_save_attempted: Mutex<Option<oneshot::Sender<()>>>,
}

#[cfg(test)]
impl LifecycleTestHooks {
    async fn pause_point(
        slot: &Mutex<Option<LifecycleTestPause>>,
        entered: oneshot::Sender<()>,
        release: oneshot::Receiver<()>,
    ) {
        *slot.lock().await = Some(LifecycleTestPause { entered, release });
    }

    async fn wait_at(slot: &Mutex<Option<LifecycleTestPause>>) {
        if let Some(pause) = slot.lock().await.take() {
            let _ = pause.entered.send(());
            let _ = pause.release.await;
        }
    }

    pub(crate) async fn hold_after_plugin_mutation(
        &self,
    ) -> (oneshot::Receiver<()>, oneshot::Sender<()>) {
        let (entered_tx, entered_rx) = oneshot::channel();
        let (release_tx, release_rx) = oneshot::channel();
        Self::pause_point(&self.after_plugin_mutation, entered_tx, release_rx).await;
        (entered_rx, release_tx)
    }

    pub(crate) async fn hold_after_uninstall_check(
        &self,
    ) -> (oneshot::Receiver<()>, oneshot::Sender<()>) {
        let (entered_tx, entered_rx) = oneshot::channel();
        let (release_tx, release_rx) = oneshot::channel();
        Self::pause_point(&self.after_uninstall_check, entered_tx, release_rx).await;
        (entered_rx, release_tx)
    }

    pub(crate) async fn watch_dashboard_save_attempt(&self) -> oneshot::Receiver<()> {
        let (attempted_tx, attempted_rx) = oneshot::channel();
        *self.dashboard_save_attempted.lock().await = Some(attempted_tx);
        attempted_rx
    }

    async fn plugin_mutation_finished(&self) {
        Self::wait_at(&self.after_plugin_mutation).await;
    }

    async fn uninstall_check_finished(&self) {
        Self::wait_at(&self.after_uninstall_check).await;
    }

    pub(crate) async fn dashboard_save_started(&self) {
        if let Some(attempted) = self.dashboard_save_attempted.lock().await.take() {
            let _ = attempted.send(());
        }
    }
}

async fn rebuild_widget_registry(
    builtin_widgets: &[Arc<dyn WidgetHandlerDyn>],
    plugins: &Arc<plugins::manager::PluginManager>,
    widget_registry: &SharedWidgetRegistry,
) -> VestaResult<()> {
    let builtins = builtin_widgets.to_vec();
    let plugins = Arc::clone(plugins);
    let next = tokio::task::spawn_blocking(move || plugins::build_registry(&builtins, &plugins))
        .await
        .map_err(|error| {
            VestaError::Plugin(format!("widget registry rebuild task failed: {error}"))
        })?;
    widget_registry.replace(next);
    Ok(())
}

async fn plugin_lifecycle_worker(
    mut requests: mpsc::Receiver<PluginRequest>,
    plugin_ops: Arc<Mutex<()>>,
    config: Arc<ConfigManager>,
    plugins: Arc<plugins::manager::PluginManager>,
    builtin_widgets: Vec<Arc<dyn WidgetHandlerDyn>>,
    widget_registry: SharedWidgetRegistry,
    #[cfg(test)] test_hooks: Arc<LifecycleTestHooks>,
) {
    while let Some(request) = requests.recv().await {
        // The worker, rather than the HTTP future, owns the complete
        // mutation/check/rebuild transaction and its serialization permit.
        let _lifecycle = Arc::clone(&plugin_ops).lock_owned().await;
        match request {
            PluginRequest::Install { url, reply } => {
                let result = match plugins.install(&url).await {
                    Ok(record) => {
                        #[cfg(test)]
                        test_hooks.plugin_mutation_finished().await;
                        rebuild_widget_registry(&builtin_widgets, &plugins, &widget_registry)
                            .await
                            .map(|()| record)
                    }
                    Err(error) => Err(error),
                };
                let _ = reply.send(result);
            }
            PluginRequest::Update { plugin_id, reply } => {
                let result = match plugins.update(&plugin_id).await {
                    Ok(record) => {
                        #[cfg(test)]
                        test_hooks.plugin_mutation_finished().await;
                        rebuild_widget_registry(&builtin_widgets, &plugins, &widget_registry)
                            .await
                            .map(|()| record)
                    }
                    Err(error) => Err(error),
                };
                let _ = reply.send(result);
            }
            PluginRequest::Uninstall { plugin_id, reply } => {
                let prefix = format!("{}{}{}", plugins::PLUGIN_NAME_PREFIX, plugin_id, ':');
                let in_use = config
                    .snapshot()
                    .items
                    .iter()
                    .any(|item| matches!(item, config::DashboardItem::Widget(widget) if widget.name.starts_with(&prefix)));
                let result = if in_use {
                    Err(VestaError::Plugin(format!(
                        "plugin '{plugin_id}' is still used by dashboard tiles; remove those tiles first"
                    )))
                } else {
                    #[cfg(test)]
                    test_hooks.uninstall_check_finished().await;
                    match plugins.uninstall(&plugin_id).await {
                        Ok(()) => {
                            #[cfg(test)]
                            test_hooks.plugin_mutation_finished().await;
                            rebuild_widget_registry(&builtin_widgets, &plugins, &widget_registry)
                                .await
                        }
                        Err(error) => Err(error),
                    }
                };
                let _ = reply.send(result);
            }
        }
    }
}

impl AppState {
    fn builtin_widgets(http_client: reqwest::Client) -> Vec<Arc<dyn WidgetHandlerDyn>> {
        vec![
            WidgetRegistry::handler(AdGuardHomeWidget::new(http_client.clone())),
            WidgetRegistry::handler(SonarrCalendarWidget::new(http_client.clone())),
            WidgetRegistry::handler(ProwlarrWidget::new(http_client.clone())),
            WidgetRegistry::handler(JellyfinRandomShowWidget::new(http_client.clone())),
            WidgetRegistry::handler(SystemStatsWidget::new()),
            WidgetRegistry::handler(WeatherWidget::new(http_client)),
            WidgetRegistry::handler(ClockWidget::new()),
            WidgetRegistry::handler(QuickLinksWidget::new()),
        ]
    }

    pub(crate) async fn install_plugin(
        &self,
        url: &str,
    ) -> VestaResult<plugins::manifest::PluginRecord> {
        let (reply, result) = oneshot::channel();
        self.plugin_requests
            .send(PluginRequest::Install {
                url: url.to_string(),
                reply,
            })
            .await
            .map_err(|_| VestaError::Plugin("plugin lifecycle worker stopped".to_string()))?;
        result
            .await
            .map_err(|_| VestaError::Plugin("plugin lifecycle worker stopped".to_string()))?
    }

    pub(crate) async fn update_plugin(
        &self,
        plugin_id: &str,
    ) -> VestaResult<plugins::manifest::PluginRecord> {
        let (reply, result) = oneshot::channel();
        self.plugin_requests
            .send(PluginRequest::Update {
                plugin_id: plugin_id.to_string(),
                reply,
            })
            .await
            .map_err(|_| VestaError::Plugin("plugin lifecycle worker stopped".to_string()))?;
        result
            .await
            .map_err(|_| VestaError::Plugin("plugin lifecycle worker stopped".to_string()))?
    }

    pub(crate) async fn uninstall_plugin(&self, plugin_id: &str) -> VestaResult<()> {
        let (reply, result) = oneshot::channel();
        self.plugin_requests
            .send(PluginRequest::Uninstall {
                plugin_id: plugin_id.to_string(),
                reply,
            })
            .await
            .map_err(|_| VestaError::Plugin("plugin lifecycle worker stopped".to_string()))?;
        result
            .await
            .map_err(|_| VestaError::Plugin("plugin lifecycle worker stopped".to_string()))?
    }

    async fn new(settings: &Settings) -> VestaResult<Arc<Self>> {
        let http_client = create_default_client()?;
        let database = Arc::new(Database::open(&settings.database_path).await?);
        let plugins = Arc::new(plugins::manager::PluginManager::new(&settings.plugins_dir)?);
        let builtin_widgets = Self::builtin_widgets(http_client.clone());
        let widget_registry =
            SharedWidgetRegistry::new(plugins::build_registry(&builtin_widgets, &plugins));

        let config = Arc::new(ConfigManager::new(
            &settings.config_path,
            widget_registry.clone(),
        )?);
        let plugin_ops = Arc::new(Mutex::new(()));
        let (plugin_requests, plugin_receiver) = mpsc::channel(PLUGIN_LIFECYCLE_QUEUE_CAPACITY);
        #[cfg(test)]
        let lifecycle_test_hooks = Arc::new(LifecycleTestHooks::default());
        let plugin_lifecycle_worker = tokio::spawn(plugin_lifecycle_worker(
            plugin_receiver,
            Arc::clone(&plugin_ops),
            Arc::clone(&config),
            Arc::clone(&plugins),
            builtin_widgets.clone(),
            widget_registry.clone(),
            #[cfg(test)]
            Arc::clone(&lifecycle_test_hooks),
        ));
        let nodes = NodeService::start(&config.snapshot().nodes, http_client.clone());
        let ping = PingService::new(http_client);
        let widget_ctx = WidgetContext {
            nodes: nodes.clone(),
        };

        Ok(Arc::new(Self {
            config,
            database,
            nodes,
            ping,
            widget_registry,
            plugins,
            plugin_ops,
            plugin_requests,
            _plugin_lifecycle_worker: plugin_lifecycle_worker,
            #[cfg(test)]
            lifecycle_test_hooks,
            widget_ctx,
            cookie_secure: settings.cookie_secure,
        }))
    }
}

fn build_router(state: Arc<AppState>, static_dir: &str) -> Router {
    let settings_routes = Router::new()
        .route(
            "/api/settings",
            get(settings_api::get_settings).put(settings_api::save_settings),
        )
        .route("/api/settings/icons", post(settings_api::upload_icon))
        .route(
            "/api/settings/dashboard",
            put(settings_api::save_dashboard_items),
        )
        .route("/api/settings/ping-test", post(settings_api::test_ping_url))
        .route(
            "/api/plugins",
            get(settings_api::list_plugins).post(settings_api::install_plugin),
        )
        .route(
            "/api/plugins/{plugin_id}",
            put(settings_api::update_plugin).delete(settings_api::uninstall_plugin),
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

/// SIGINT/SIGTERM end the process. TOML settings are saved atomically on request.
async fn shutdown() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut term = signal(SignalKind::terminate()).expect("install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = term.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await.ok();
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "vesta=info".into()))
        .init();

    let settings = Settings::from_env();
    let state = match AppState::new(&settings).await {
        Ok(state) => state,
        Err(e) => {
            tracing::error!(error = %e, "startup failed");
            std::process::exit(1);
        }
    };

    let app = build_router(state.clone(), &settings.static_dir);
    let listener = tokio::net::TcpListener::bind(&settings.bind_addr)
        .await
        .expect("bind listen address");

    tracing::info!(addr = %settings.bind_addr, "vesta listening");
    tracing::debug!(
        database = %settings.database_path,
        widgets = ?state.widget_registry.current().names(),
        "registered widgets"
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown())
        .await
        .expect("server run");

    state.nodes.shutdown();
    tracing::info!("vesta stopped");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_registry_exposes_expected_sizes_and_catalog_visibility() {
        let builtins = AppState::builtin_widgets(create_default_client().unwrap());
        let mut registry = WidgetRegistry::new();
        for handler in &builtins {
            registry = registry.register_shared(Arc::clone(handler));
        }

        for (name, expected) in [
            ("Clock", (4, 2)),
            ("QuickLinks", (3, 2)),
            ("SystemStats", (6, 4)),
            ("Weather", (4, 3)),
            ("SonarrCalendar", (4, 4)),
            ("Prowlarr", (4, 2)),
            ("AdGuard", (4, 2)),
            ("JellyfinRandomShow", (4, 3)),
        ] {
            assert_eq!(registry.default_size(name), Some(expected), "{name}");
        }

        assert!(registry.get("SystemStats").is_some());
        assert!(!registry.get("SystemStats").unwrap().catalog_visible());
        assert!(
            !registry
                .catalog_descriptors()
                .iter()
                .any(|d| d.name == "SystemStats")
        );
    }

    #[tokio::test]
    async fn http_router_protects_admin_routes_and_completes_plugin_lifecycle() {
        let sandbox = crate::plugins::test_fixtures::temp_dir("vesta-http-router");
        let static_dir = sandbox.path().join("static");
        std::fs::create_dir_all(&static_dir).unwrap();
        let settings = Settings {
            config_path: sandbox.path().join("config.toml").display().to_string(),
            database_path: sandbox.path().join("vesta.db").display().to_string(),
            plugins_dir: sandbox.path().join("plugins").display().to_string(),
            bind_addr: "127.0.0.1:0".to_string(),
            static_dir: static_dir.display().to_string(),
            cookie_secure: false,
        };
        let state = AppState::new(&settings).await.unwrap();
        let router = build_router(state.clone(), &settings.static_dir);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });

        let base = format!("http://{addr}");
        let client = reqwest::Client::new();

        // Public health check is reachable without auth; admin routes reject.
        assert_eq!(
            client
                .get(format!("{base}/api/health"))
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::OK
        );
        assert_eq!(
            client
                .get(format!("{base}/api/settings"))
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            client
                .get(format!("{base}/api/plugins"))
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::UNAUTHORIZED
        );

        // First-run setup authenticates and issues an HttpOnly session cookie.
        let setup_res = client
            .post(format!("{base}/api/auth/setup"))
            .json(&serde_json::json!({ "password": "correct-horse-battery" }))
            .send()
            .await
            .unwrap();
        assert_eq!(setup_res.status(), reqwest::StatusCode::OK);
        let cookie_header = setup_res
            .headers()
            .get(reqwest::header::SET_COOKIE)
            .expect("setup sets session cookie")
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_string();

        // Catalog excludes legacy SystemStats before plugin install.
        let settings_before: serde_json::Value = client
            .get(format!("{base}/api/settings"))
            .header(reqwest::header::COOKIE, &cookie_header)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let names_before: Vec<_> = settings_before["data"]["widgets"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|d| d["name"].as_str())
            .collect();
        assert!(!names_before.contains(&"SystemStats"));

        // Install hello-world plugin from a local git repo fixture.
        let repo = crate::plugins::test_fixtures::git_fixture_repo(
            &crate::plugins::test_fixtures::fixture_dir("hello-world"),
            "http-lifecycle",
        );
        let install_res = client
            .post(format!("{base}/api/plugins"))
            .header(reqwest::header::COOKIE, &cookie_header)
            .json(&serde_json::json!({ "url": repo.path().display().to_string() }))
            .send()
            .await
            .unwrap();
        assert_eq!(install_res.status(), reqwest::StatusCode::OK);
        let install_body: serde_json::Value = install_res.json().await.unwrap();
        assert_eq!(install_body["data"]["id"], "hello-world");

        // Update pulls a fresh commit over the same endpoint cleanly.
        let update_res = client
            .put(format!("{base}/api/plugins/hello-world"))
            .header(reqwest::header::COOKIE, &cookie_header)
            .send()
            .await
            .unwrap();
        assert_eq!(update_res.status(), reqwest::StatusCode::OK);

        // Asset serving works unauthenticated with security headers and rejects traversal.
        let asset_res = client
            .get(format!("{base}/api/plugins/hello-world/assets/index.html"))
            .send()
            .await
            .unwrap();
        assert_eq!(asset_res.status(), reqwest::StatusCode::OK);
        assert_eq!(
            asset_res
                .headers()
                .get("x-content-type-options")
                .and_then(|v| v.to_str().ok()),
            Some("nosniff")
        );
        assert_eq!(
            asset_res
                .headers()
                .get("cache-control")
                .and_then(|v| v.to_str().ok()),
            Some("no-cache")
        );
        assert_eq!(
            asset_res
                .headers()
                .get("content-security-policy")
                .and_then(|v| v.to_str().ok()),
            Some("frame-ancestors 'self'")
        );
        assert!(
            asset_res
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .starts_with("text/html")
        );
        assert_eq!(
            client
                .get(format!(
                    "{base}/api/plugins/hello-world/assets/..%2fvesta-plugin.toml"
                ))
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::NOT_FOUND
        );

        // Bind a counter plugin tile and verify widget payload + uninstall guard.
        let save_tile_res = client
            .put(format!("{base}/api/settings/dashboard"))
            .header(reqwest::header::COOKIE, &cookie_header)
            .json(&serde_json::json!({
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
            .send()
            .await
            .unwrap();
        assert_eq!(save_tile_res.status(), reqwest::StatusCode::OK);

        let widget_res: serde_json::Value = client
            .get(format!(
                "{base}/api/widgets/Plugin:hello-world:counter?id=counter-1"
            ))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(widget_res["data"]["widget"]["plugin_id"], "hello-world");
        assert_eq!(widget_res["data"]["widget"]["widget_id"], "counter");
        assert_eq!(widget_res["data"]["widget"]["config"]["label"], "Hits");

        let blocked_delete = client
            .delete(format!("{base}/api/plugins/hello-world"))
            .header(reqwest::header::COOKIE, &cookie_header)
            .send()
            .await
            .unwrap();
        assert_eq!(blocked_delete.status(), reqwest::StatusCode::BAD_REQUEST);

        // Removing the tile unblocks uninstall and returns {"removed": "hello-world"}.
        assert_eq!(
            client
                .put(format!("{base}/api/settings/dashboard"))
                .header(reqwest::header::COOKIE, &cookie_header)
                .json(&serde_json::json!({ "items": [] }))
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::OK
        );
        let uninstall_res = client
            .delete(format!("{base}/api/plugins/hello-world"))
            .header(reqwest::header::COOKIE, &cookie_header)
            .send()
            .await
            .unwrap();
        assert_eq!(uninstall_res.status(), reqwest::StatusCode::OK);
        let uninstall_body: serde_json::Value = uninstall_res.json().await.unwrap();
        assert_eq!(uninstall_body["data"]["removed"], "hello-world");

        // Public dashboard endpoints remain healthy.
        assert_eq!(
            client
                .get(format!("{base}/api/dashboard"))
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::OK
        );
        assert_eq!(
            client
                .get(format!("{base}/api/dashboard/services"))
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::OK
        );

        state.nodes.shutdown();
        server.abort();
    }
}
