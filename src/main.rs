use lazy_static::lazy_static;
use std::{collections::HashMap, process::exit, sync::Arc};

use axum::{routing::get, Extension, Router};
use indexmap::IndexMap;
use maud::{html, Markup, DOCTYPE};
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

lazy_static! {
    static ref COLUMN_CSS_MAPPING: HashMap<u8, &'static str> = HashMap::from([
        (1, "grid-cols-1 w-[8rem]"),
        (2, "grid-cols-2 w-[16rem]"),
        (3, "grid-cols-3 w-[24rem]"),
        (4, "grid-cols-4 w-[32rem]"),
        (5, "grid-cols-5 w-[40rem]"),
        (6, "grid-cols-6 w-[48rem]"),
    ]);
}

fn head() -> Markup {
    html! {
        (DOCTYPE)
        html lang="en"
        head {
            meta charset="utf-8";
            meta name="viewport" content="width=device-width, initial-scale=1.0";
            link rel="stylesheet" type="text/css" href="/style.css";
            title { "Vesta" }
        }
    }
}

fn group(group_config: &GroupConfig) -> Markup {
    let column_class = COLUMN_CSS_MAPPING
        .get(&group_config.columns)
        .unwrap_or_else(|| {
            eprintln!(
                "Group '{}' column value is not valid.",
                group_config.columns
            );
            exit(1);
        });

    html! {
        div.container."mt-5".(column_class) {
            p { (group_config.name) }
        }
    }
}

async fn dashboard(Extension(state): Extension<Arc<AppState>>) -> Markup {
    html! {
        (head())
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
