use config::{load_config, Dashboard};
use ping::ping_handler;
use std::{process::exit, sync::Arc};
use templates::dashboard;
use widgets::sonarr_calendar::sonarr_calendar_handler;

use axum::{routing::get, Extension, Router};
use tower_http::services::ServeDir;

mod config;
mod ping;
mod templates;
mod widgets;

#[derive(Clone)]
struct AppState {
    config: Dashboard,
}

#[tokio::main]
async fn main() {
    let config_path = "./config/vesta.toml";
    let config = match load_config(config_path) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Error when loading config: {}", e);
            exit(1);
        }
    };

    let state = Arc::new(AppState { config });

    // build our application with a single route
    let app = Router::new()
        .route("/api/sonarr-calendar", get(sonarr_calendar_handler))
        .route("/api/ping", get(ping_handler))
        .route("/", get(dashboard))
        .nest_service("/static", ServeDir::new("static"))
        .layer(Extension(state));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
