use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PingConfig {
    pub url: String,
}

/// Widget-specific JSON settings are opaque to the dashboard and interpreted
/// by the widget implementation that owns them.
pub type WidgetConfig = HashMap<String, serde_json::Value>;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Widget {
    pub name: String,
    #[serde(default)]
    pub config: Option<WidgetConfig>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Service {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    pub href: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub widget: Option<Widget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ping: Option<PingConfig>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Group {
    pub name: String,
    #[serde(default)]
    pub services: Vec<Service>,
}

/// Dashboard configuration normalized for runtime services and APIs.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Dashboard {
    #[serde(default)]
    pub nodes: Vec<crate::metrics::node::NodeConfig>,
    #[serde(flatten)]
    pub groups: IndexMap<String, Group>,
}

/// One human-editable TOML document contains all dashboard and app settings.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AppConfig {
    #[serde(default)]
    pub nodes: Vec<crate::metrics::node::NodeConfig>,
    #[serde(default)]
    pub app: AppSettings,
    #[serde(default)]
    pub groups: IndexMap<String, Group>,
}

impl AppConfig {
    pub fn dashboard(&self) -> Dashboard {
        Dashboard {
            nodes: self.nodes.clone(),
            groups: self.groups.clone(),
        }
    }

    pub fn from_dashboard(dashboard: &Dashboard, app: &AppSettings) -> Self {
        Self {
            nodes: dashboard.nodes.clone(),
            app: app.clone(),
            groups: dashboard.groups.clone(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AppSettings {
    #[serde(default = "default_dashboard_title")]
    pub dashboard_title: String,
    #[serde(default)]
    pub theme: Theme,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            dashboard_title: default_dashboard_title(),
            theme: Theme::default(),
        }
    }
}

fn default_dashboard_title() -> String {
    "Server Monitor".to_string()
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
