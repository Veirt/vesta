use lazy_static::lazy_static;
use std::{collections::HashMap, process::exit, sync::Arc};

use axum::{routing::get, Extension, Router};
use indexmap::IndexMap;
use maud::{html, Markup, DOCTYPE};
use serde::Deserialize;
use tower_http::services::ServeDir;

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
    static ref COLUMN_CSS_MAPPING: HashMap<u8, (&'static str, &'static str)> = HashMap::from([
        (1, ("grid-cols-1", "w-[8rem]")),
        (2, ("grid-cols-2", "w-[16rem]")),
        (3, ("grid-cols-3", "w-[24rem]")),
        (4, ("grid-cols-4", "w-[32rem]")),
        (5, ("grid-cols-5", "w-[40rem]")),
        (6, ("grid-cols-6", "w-[48rem]")),
    ]);
    static ref CARD_WIDTH_CSS_MAPPING: HashMap<u8, &'static str> = HashMap::from([
        (1, "md:min-w-[7rem] col-span-1"),
        (2, "md:min-w-[15rem] col-span-2"),
        (3, "md:min-w-[23rem] col-span-3"),
        (4, "md:min-w-[31rem] col-span-4"),
        (5, "md:min-w-[39rem] col-span-5"),
        (6, "md:min-w-[47rem] col-span-6"),
    ]);
    static ref CARD_HEIGHT_CSS_MAPPING: HashMap<u8, &'static str> = HashMap::from([
        (1, "min-h-[7rem] max-h-[7rem] row-span-1"),
        (2, "min-h-[15rem] max-h-[15rem] row-span-2"),
        (3, "min-h-[23rem] max-h-[23rem] row-span-3"),
        (4, "min-h-[31rem] max-h-[31rem] row-span-4"),
        (5, "min-h-[39rem] max-h-[39rem] row-span-5"),
        (6, "min-h-[47rem] max-h-[47rem] row-span-6"),
    ]);
}

fn head() -> Markup {
    html! {
        (DOCTYPE)
        html lang="en"
        head {
            meta charset="utf-8";
            meta name="viewport" content="width=device-width, initial-scale=1.0";
            link rel="stylesheet" type="text/css" href="/static/style.css";
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
        div.container."mt-5".(column_class.1) {
            p.block."ml-4"."font-bold" { (group_config.name) }
            div.grid.(column_class.0) {
                @for service in &group_config.services {
                    (service_card(service))
                }
            }
        }
    }
}

fn service_card(service_config: &Service) -> Markup {
    let img_src = service_config.img_src.as_deref().unwrap_or_default();
    let href = service_config.href.as_deref().unwrap_or_default();
    let card_width_class = CARD_WIDTH_CSS_MAPPING
        .get(&service_config.width.unwrap_or(1))
        .unwrap_or_else(|| {
            eprintln!("Card '{}' width value is not valid.", service_config.title);
            exit(1);
        });
    let card_height_class = CARD_HEIGHT_CSS_MAPPING
        .get(&service_config.height.unwrap_or(1))
        .unwrap_or_else(|| {
            eprintln!("Card '{}' height value is not valid.", service_config.title);
            exit(1);
        });

    html! {
        a href=(href) class=(format!("flex flex-col justify-center items-center text-xs bg-black-2 rounded-xl py-2 m-2 hover:scale-105 duration-150 {} {}", card_width_class, card_height_class)) {
            img class="object-contain my-3 w-[2rem] h-[2rem]" src=(img_src) alt=(service_config.title);
            p class="text-center" { (service_config.title) }
        }
    }
}

async fn dashboard(Extension(state): Extension<Arc<AppState>>) -> Markup {
    html! {
        (head())
        body.flex."justify-center"."items-center"."min-h-screen"."text-white"."bg-black" {
            div.container.flex."flex-row"."flex-wrap"."justify-center"."h-screen"."sm:justify-start"  {
                @for group_config in state.config.groups.values() {
                    (group(group_config))
                }
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
        .nest_service("/static", ServeDir::new("static"))
        .layer(Extension(state));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
