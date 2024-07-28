use std::{process::exit, sync::Arc};

use axum::{routing::get, Extension, Router};
use indexmap::IndexMap;
use maud::{html, Markup};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
struct ServiceWidget {
    name: String,
    url: Option<String>,
    key: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct PingConfig {
    url: String,
}

#[derive(Clone, Debug, Deserialize)]
struct Service {
    title: String,
    href: Option<String>,
    img_src: Option<String>,
    width: Option<u8>,
    height: Option<u8>,
    widget: Option<ServiceWidget>,
    ping: Option<PingConfig>,
}

#[derive(Clone, Debug, Deserialize)]
struct GroupConfig {
    name: String,
    columns: u8,
    services: Vec<Service>,
}

#[derive(Clone, Deserialize)]
struct VestaConfig {
    #[serde(flatten)]
    groups: IndexMap<String, GroupConfig>,
}

mod config {
    use std::fs;

    use crate::VestaConfig;

    pub fn load(path: &str) -> Result<VestaConfig, Box<dyn std::error::Error>> {
        let contents = fs::read_to_string(path)?;
        let parsed: crate::VestaConfig = toml::from_str(&contents)?;

        Ok(parsed)
    }
}

#[derive(Clone)]
struct AppState {
    config: VestaConfig,
}

fn group(group_config: &GroupConfig) -> Markup {
    html! {
        div.container."mt-5" {
            p { (group_config.name) }
        }
    }
}

async fn dashboard(Extension(state): Extension<Arc<AppState>>) -> Markup {
    html! {
        ul {
            @for group_config in state.config.groups.values() {
                (group(group_config))
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let config_path = "./config/vesta.toml";
    let config = match config::load(config_path) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Error when loading config: {}", e);
            exit(1);
        }
    };

    let state = Arc::new(AppState { config });

    // build our application with a single route
    let app = Router::new()
        .route("/", get(dashboard))
        .layer(Extension(state));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
