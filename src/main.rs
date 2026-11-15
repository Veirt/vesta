mod api;
mod auth;
mod config;
mod config_manager;
mod database;
mod error;
mod http_client;
mod metrics;
mod ping;
mod plugins;
mod response;
mod services;
mod settings_api;
mod templates;
mod widget_config;
mod widget_system;
mod widgets;

use std::sync::Arc;

use axum::Router;
use axum::extract::{DefaultBodyLimit, Extension, Path, Query};
use axum::middleware;
use axum::response::IntoResponse;
use axum::routing::{delete, get, post, put};
use tower_http::compression::CompressionLayer;
use tower_http::services::ServeDir;
use tracing_subscriber::EnvFilter;

use config_manager::ConfigManager;
use database::Database;
use error::{VestaError, VestaResult};
use http_client::create_default_client;
use metrics::service::NodeService;
use ping::ping_handler;
use response::jsend;
use services::ping_service::PingService;
use templates::dashboard;
use widget_system::{WidgetContext, WidgetQuery, WidgetRegistry};
use widgets::clock_widget::ClockWidget;
use widgets::jellyfin_random_show_widget::JellyfinRandomShowWidget;
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
    pub(crate) widget_registry: plugins::SharedWidgetRegistry,
    pub(crate) plugins: Arc<plugins::manager::PluginManager>,
    pub(crate) http_client: reqwest::Client,
    pub(crate) widget_ctx: WidgetContext,
    pub(crate) cookie_secure: bool,
}

impl AppState {
    fn builtin_registry(http_client: reqwest::Client) -> WidgetRegistry {
        WidgetRegistry::new()
            .register(SonarrCalendarWidget::new(http_client.clone()))
            .register(JellyfinRandomShowWidget::new(http_client.clone()))
            .register(SystemStatsWidget::new())
            .register(WeatherWidget::new(http_client.clone()))
            .register(ClockWidget::new())
            .register(QuickLinksWidget::new())
    }

    /// Rebuild the widget registry from the built-ins and every installed
    /// plugin. Called at startup and whenever a plugin is installed or
    /// removed, so both request handling and config validation switch over.
    pub(crate) fn rebuild_widget_registry(&self) {
        self.widget_registry.replace(plugins::build_registry(
            Self::builtin_registry(self.http_client.clone()),
            &self.plugins,
        ));
    }

    async fn new(settings: &Settings) -> VestaResult<Arc<Self>> {
        let http_client = create_default_client()?;
        let database = Arc::new(Database::open(&settings.database_path).await?);
        let plugins = Arc::new(plugins::manager::PluginManager::new(&settings.plugins_dir)?);
        let widget_registry = plugins::SharedWidgetRegistry::new(plugins::build_registry(
            Self::builtin_registry(http_client.clone()),
            &plugins,
        ));

        let config = Arc::new(ConfigManager::new(
            &settings.config_path,
            widget_registry.clone(),
        )?);
        let nodes = NodeService::start(&config.snapshot().nodes, http_client.clone());
        let ping = PingService::new(http_client.clone());
        let widget_ctx = WidgetContext {
            nodes: nodes.clone(),
            config: config.clone(),
        };

        Ok(Arc::new(Self {
            config,
            database,
            nodes,
            ping,
            widget_registry,
            plugins,
            http_client,
            widget_ctx,
            cookie_secure: settings.cookie_secure,
        }))
    }
}

async fn widget_handler(
    Path(widget_name): Path<String>,
    Query(query): Query<WidgetQuery>,
    Extension(state): Extension<Arc<AppState>>,
) -> Result<impl IntoResponse, VestaError> {
    let data = state
        .widget_registry
        .current()
        .handle_request(&widget_name, state.widget_ctx.clone(), query)
        .await?;
    Ok(jsend::success(data))
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
            delete(settings_api::uninstall_plugin),
        )
        .route("/api/nodes", get(api::list_nodes))
        .route("/api/services", get(api::get_service))
        .route("/api/widget", get(api::get_widget))
        .route(
            "/api/widgets/{widget_name}/options",
            post(api::widget_options),
        )
        .route_layer(middleware::from_fn(auth::require_admin));

    Router::new()
        .route("/api/widgets/{widget_name}", get(widget_handler))
        .route("/api/ping", get(ping_handler))
        .route("/api/dashboard", get(api::dashboard))
        .route("/api/dashboard/services", get(api::dashboard_services))
        .route("/api/nodes/events", get(api::node_events))
        .route("/api/nodes/{node_id}/stats", get(api::node_stats))
        .route("/api/health", get(api::health))
        .route("/api/auth/status", get(auth::status))
        .route("/api/auth/setup", post(auth::setup))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/logout", post(auth::logout))
        .route("/api/icons/{icon_id}", get(settings_api::get_icon))
        .merge(settings_routes)
        .route("/", get(dashboard))
        .route("/settings", get(dashboard))
        .route(
            "/api/plugins/{plugin_id}/assets/{*path}",
            get(settings_api::plugin_asset),
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
