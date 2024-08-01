use indexmap::IndexMap;
use serde::Deserialize;
use std::{collections::HashMap, fs};

pub fn load_config(path: &str) -> Result<Dashboard, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let parsed: Dashboard = toml::from_str(&contents)?;

    Ok(parsed)
}

pub fn get_widget_info<'a>(config: &'a Dashboard, group: &str, title: &str) -> Option<&'a Widget> {
    config.groups.get(group).and_then(|group| {
        group
            .services
            .iter()
            .find(|service| service.title == title)
            .and_then(|service| service.widget.as_ref())
    })
}

#[derive(Clone, Debug, Deserialize)]
pub struct PingConfig {
    pub url: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Widget {
    pub name: String,
    pub config: Option<HashMap<String, String>>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Service {
    pub title: String,
    pub href: Option<String>,
    pub img_src: Option<String>,
    pub width: Option<u8>,
    pub height: Option<u8>,
    pub widget: Option<Widget>,
    pub ping: Option<PingConfig>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Group {
    pub name: String,
    pub columns: u8,
    pub services: Vec<Service>,
}

#[derive(Clone, Deserialize)]
pub struct Dashboard {
    #[serde(flatten)]
    pub groups: IndexMap<String, Group>,
}
