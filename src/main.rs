use config::Dashboard;
use config_manager::ConfigManager;
use error::{VestaError, VestaResult};
use http_client::create_default_client;
use ping::ping_handler;
use reqwest::Client;
use std::{process::exit, sync::Arc};
use templates::dashboard;
use widget_system::WidgetRegistry;
use widgets::clock_widget::ClockWidget;
use widgets::quick_links_widget::QuickLinksWidget;
use widgets::sonarr_calendar_widget::SonarrCalendarWidget;
use widgets::system_stats_widget::SystemStatsWidget;
use widgets::weather_widget::WeatherWidget;

use axum::{
    Router,
    extract::{Extension, Path, Query},
    response::IntoResponse,
    routing::{get, post},
};
use tower_http::services::ServeDir;

mod api;
mod config;
mod config_manager;
mod error;
mod http_client;
mod ping;
mod response;
mod templates;
mod widget_system;
mod widgets;

pub struct AppState {
    config_manager: Arc<ConfigManager>,
    http_client: Client,
    widget_registry: Arc<WidgetRegistry>,
}

impl AppState {
    pub fn new(config_path: &str) -> VestaResult<Arc<Self>> {
        let widget_registry = Arc::new(
            WidgetRegistry::new()
                .register(SonarrCalendarWidget::new())
                .register(SystemStatsWidget::new())
                .register(WeatherWidget::new())
                .register(ClockWidget::new())
                .register(QuickLinksWidget::new()),
        );

        let config_manager = Arc::new(ConfigManager::new(config_path, widget_registry.clone())?);
        let http_client = create_default_client()?;

        Ok(Arc::new(Self {
            config_manager,
            http_client,
            widget_registry,
        }))
    }

    pub fn reload_config(&self) -> VestaResult<()> {
        self.config_manager.reload_config()
    }

    pub fn get_config(&self) -> VestaResult<Dashboard> {
        self.config_manager.get_config()
    }

    pub fn get_http_client(&self) -> &Client {
        &self.http_client
    }

    pub fn get_widget_registry(&self) -> &WidgetRegistry {
        &self.widget_registry
    }

    pub fn get_config_manager(&self) -> &ConfigManager {
        &self.config_manager
    }
}

async fn widget_handler(
    Path(widget_name): Path<String>,
    Query(query): Query<widget_system::WidgetQuery>,
    Extension(state): Extension<Arc<AppState>>,
) -> Result<impl IntoResponse, VestaError> {
    let state_clone = Arc::clone(&state);
    state
        .widget_registry
        .handle_widget_request(&widget_name, state_clone, query)
        .await
}

#[tokio::main]
async fn main() {
    let config_path = "./config/vesta.toml";
    let state = match AppState::new(config_path) {
        Ok(state) => state,
        Err(e) => {
            eprintln!("Error when loading config: {}", e);
            exit(1);
        }
    };

    let app = Router::new()
        .route("/api/widgets/{widget_name}", get(widget_handler))
        .route("/api/ping", get(ping_handler))
        .route("/api/health", get(api::health))
        .route("/api/services", get(api::list_services))
        .route("/api/service", get(api::get_service))
        .route("/api/widget", get(api::get_widget))
        .route("/api/config/validate", get(api::validate_config))
        .route("/api/config/reload", post(api::reload_config))
        .route("/", get(dashboard))
        .nest_service("/static", ServeDir::new("static"))
        .layer(Extension(state));

    let address = "0.0.0.0:3000";

    println!("Listening on http://{address}");
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
