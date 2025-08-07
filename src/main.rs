use config::{load_config, Dashboard};
use error::{VestaError, VestaResult};
use ping::ping_handler;
use std::{
    process::exit,
    sync::{Arc, RwLock},
};
use templates::dashboard;
use widgets::sonarr_calendar::sonarr_calendar_handler;

use axum::{routing::get, Extension, Router};
use tower_http::services::ServeDir;

mod config;
mod error;
mod ping;
mod templates;
mod widgets;

pub struct AppState {
    config: RwLock<Dashboard>,
    config_path: String,
}

impl AppState {
    pub fn new(config_path: &str) -> VestaResult<Arc<Self>> {
        let config = load_config(config_path)?;
        Ok(Arc::new(Self {
            config: RwLock::new(config),
            config_path: config_path.to_string(),
        }))
    }

    pub fn reload_config(&self) -> VestaResult<()> {
        let new_config = load_config(&self.config_path)?;
        let mut config = self
            .config
            .write()
            .map_err(|e| VestaError::Internal(format!("Failed to acquire write lock: {}", e)))?;
        *config = new_config;
        Ok(())
    }

    pub fn get_config(&self) -> Result<Dashboard, VestaError> {
        self.config
            .read()
            .map_err(|e| VestaError::Internal(format!("Failed to acquire read lock: {}", e)))
            .map(|config| config.clone())
    }
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
        .route("/api/sonarr-calendar", get(sonarr_calendar_handler))
        .route("/api/ping", get(ping_handler))
        .route("/", get(dashboard))
        .nest_service("/static", ServeDir::new("static"))
        .layer(Extension(state));

    let address = "0.0.0.0:3000";

    println!("Listening on http://{address}");
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
