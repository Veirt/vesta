use indexmap::IndexMap;
use serde::Deserialize;
use std::{collections::HashMap, fs, path::Path};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Failed to parse TOML: {0}")]
    TomlError(#[from] toml::de::Error),
}

pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Dashboard, ConfigError> {
    let contents = fs::read_to_string(path)?;
    let parsed: Dashboard = toml::from_str(&contents)?;
    Ok(parsed)
}

pub fn get_service_info<'a>(
    config: &'a Dashboard,
    group: &str,
    title: &str,
) -> Option<&'a Service> {
    config
        .groups
        .get(group)
        .and_then(|group| group.services.iter().find(|service| service.title == title))
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
    #[serde(alias = "imgSrc")]
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

impl Dashboard {
    pub fn get_service(&self, group: &str, title: &str) -> Option<&Service> {
        get_service_info(self, group, title)
    }

    pub fn get_widget(&self, group: &str, title: &str) -> Option<&Widget> {
        get_widget_info(self, group, title)
    }
}
