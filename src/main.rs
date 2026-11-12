mod api;
mod config;
mod config_manager;
mod error;
mod http_client;
mod metrics;
mod ping;
mod response;
mod services;
mod templates;
mod widget_config;
mod widget_system;
mod widgets;

use std::sync::Arc;

use axum::Router;
use axum::extract::{Extension, Path, Query};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use tower_http::compression::CompressionLayer;
use tower_http::services::ServeDir;
use tracing_subscriber::EnvFilter;

use config_manager::ConfigManager;
use error::{VestaError, VestaResult};
use http_client::create_default_client;
use metrics::service::NodeService;
use ping::ping_handler;
use services::ping_service::PingService;
use templates::dashboard;
use widget_system::{WidgetContext, WidgetQuery, WidgetRegistry};
use widgets::clock_widget::ClockWidget;
use widgets::quick_links_widget::QuickLinksWidget;
use widgets::sonarr_calendar_widget::SonarrCalendarWidget;
use widgets::system_stats_widget::SystemStatsWidget;
use widgets::weather_widget::WeatherWidget;

/// Runtime settings, all overridable for device deployments where editing a
/// TOML file is harder than setting an env var.
struct Settings {
    config_path: String,
    bind_addr: String,
    static_dir: String,
}

impl Settings {
    fn from_env() -> Self {
        Self {
            config_path: env_or("VESTA_CONFIG", "./config/vesta.toml"),
            bind_addr: env_or("VESTA_BIND", "0.0.0.0:3000"),
            static_dir: env_or("VESTA_STATIC_DIR", "static"),
        }
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

pub struct AppState {
    config: Arc<ConfigManager>,
    nodes: Arc<NodeService>,
    ping: PingService,
    widget_registry: Arc<WidgetRegistry>,
    widget_ctx: WidgetContext,
}

impl AppState {
    fn new(settings: &Settings) -> VestaResult<Arc<Self>> {
        let http_client = create_default_client()?;
        let widget_registry = Arc::new(
            WidgetRegistry::new()
                .register(SonarrCalendarWidget::new(http_client.clone()))
                .register(SystemStatsWidget::new())
                .register(WeatherWidget::new(http_client.clone()))
                .register(ClockWidget::new())
                .register(QuickLinksWidget::new()),
        );

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
            nodes,
            ping,
            widget_registry,
            widget_ctx,
        }))
    }
}

async fn widget_handler(
    Path(widget_name): Path<String>,
    Query(query): Query<WidgetQuery>,
    Extension(state): Extension<Arc<AppState>>,
) -> Result<impl IntoResponse, VestaError> {
    state
        .widget_registry
        .handle_request(&widget_name, state.widget_ctx.clone(), query)
        .await
}

fn build_router(state: Arc<AppState>, static_dir: &str) -> Router {
    Router::new()
        .route("/api/widgets/{widget_name}", get(widget_handler))
        .route("/api/ping", get(ping_handler))
        .route("/api/nodes", get(api::list_nodes))
        .route("/api/nodes/{node_id}/stats", get(api::node_stats))
        .route("/api/health", get(api::health))
        .route("/api/services", get(api::get_service))
        .route("/api/widget", get(api::get_widget))
        .route("/api/config/validate", get(api::validate_config))
        .route("/api/config/reload", post(api::reload_config))
        .route("/", get(dashboard))
        .nest_service("/static", ServeDir::new(static_dir))
        .layer(CompressionLayer::new())
        .layer(Extension(state))
}

/// SIGINT/SIGTERM end the process; SIGHUP reloads the config in place.
async fn shutdown_or_reload(config: Arc<ConfigManager>) -> Result<(), VestaError> {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut term = signal(SignalKind::terminate()).expect("install SIGTERM handler");
        let mut hangup = signal(SignalKind::hangup()).expect("install SIGHUP handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = term.recv() => {}
            _ = hangup.recv() => {
                if let Err(e) = config.reload() {
                    tracing::error!(error = %e, "SIGHUP reload failed");
                }
                Box::pin(shutdown_or_reload(config)).await?;
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = &config;
        tokio::signal::ctrl_c().await.ok();
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "vesta=info".into()))
        .init();

    let settings = Settings::from_env();
    let state = match AppState::new(&settings) {
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
        config = %settings.config_path,
        widgets = ?state.widget_registry.names(),
        "registered widgets"
    );

    let reload_state = state.config.clone();
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            if let Err(e) = shutdown_or_reload(reload_state).await {
                tracing::error!(error = %e, "shutdown handler error");
            }
        })
        .await
        .expect("server run");

    state.nodes.shutdown();
    tracing::info!("vesta stopped");
}
