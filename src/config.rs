use std::collections::HashMap;
use std::fs;
use std::path::Path;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::error::ConfigError;

pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Dashboard, ConfigError> {
    let contents = fs::read_to_string(path)?;
    let parsed: Dashboard = toml::from_str(&contents)?;
    Ok(parsed)
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PingConfig {
    pub url: String,
}

/// Raw widget options exactly as written in `widget.config`. TOML values may
/// be strings, numbers, booleans, arrays or tables; each widget's typed
/// `Config` struct decides what it accepts. Dotted keys
/// (`widget.config.foo.bar`) nest, matching the pre-refactor behavior.
pub type WidgetConfig = HashMap<String, serde_json::Value>;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Widget {
    pub name: String,
    #[serde(default)]
    pub config: Option<WidgetConfig>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Group {
    pub name: String,
    pub columns: u8,
    pub services: Vec<Service>,
}

/// Monitoring targets, declared as `[[nodes]]` in vesta.toml. Each is polled
/// on its own interval; widgets and `/api/nodes/*` read their latest stats.
#[derive(Clone, Debug, Deserialize)]
pub struct Dashboard {
    #[serde(default)]
    pub nodes: Vec<crate::metrics::node::NodeConfig>,
    #[serde(flatten)]
    pub groups: IndexMap<String, Group>,
}

impl Dashboard {
    pub fn service(&self, group: &str, title: &str) -> Option<&Service> {
        self.groups
            .get(group)?
            .services
            .iter()
            .find(|s| s.title == title)
    }

    pub fn widget(&self, group: &str, title: &str) -> Option<&Widget> {
        self.service(group, title).and_then(|s| s.widget.as_ref())
    }

    pub fn services(&self) -> impl Iterator<Item = (&str, &str, &Service)> {
        self.groups.iter().flat_map(|(gid, g)| {
            g.services
                .iter()
                .map(move |s| (gid.as_str(), g.name.as_str(), s))
        })
    }
}
