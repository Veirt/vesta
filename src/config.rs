use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PingConfig {
    pub url: String,
}

pub type WidgetConfig = HashMap<String, serde_json::Value>;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Service {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    pub href: Option<String>,
    #[serde(default = "default_show_destination")]
    pub show_destination: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ping: Option<PingConfig>,
}

fn default_show_destination() -> bool {
    true
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DashboardGroup {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u8>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Widget {
    pub id: String,
    pub title: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(default)]
    pub config: Option<WidgetConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u8>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DashboardItem {
    Service(Service),
    Widget(Widget),
}

impl DashboardItem {
    pub fn id(&self) -> &str {
        match self {
            Self::Service(service) => &service.id,
            Self::Widget(widget) => &widget.id,
        }
    }

    pub fn group(&self) -> Option<&str> {
        match self {
            Self::Service(service) => service.group.as_deref(),
            Self::Widget(widget) => widget.group.as_deref(),
        }
    }

    pub(crate) fn layout(&self) -> (Option<u8>, Option<u16>, Option<u8>, Option<u8>) {
        match self {
            Self::Service(service) => (service.x, service.y, service.width, service.height),
            Self::Widget(widget) => (widget.x, widget.y, widget.width, widget.height),
        }
    }

    pub(crate) fn set_layout(&mut self, x: u8, y: u16, width: u8, height: u8) {
        match self {
            Self::Service(service) => {
                service.x = Some(x);
                service.y = Some(y);
                service.width = Some(width);
                service.height = Some(height);
            }
            Self::Widget(widget) => {
                widget.x = Some(x);
                widget.y = Some(y);
                widget.width = Some(width);
                widget.height = Some(height);
            }
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Dashboard {
    #[serde(default)]
    pub nodes: Vec<crate::metrics::node::NodeConfig>,
    #[serde(default)]
    pub items: Vec<DashboardItem>,
    #[serde(default)]
    pub groups: Vec<DashboardGroup>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AppConfig {
    #[serde(default)]
    pub nodes: Vec<crate::metrics::node::NodeConfig>,
    #[serde(default)]
    pub app: AppSettings,
    #[serde(default)]
    pub items: Vec<DashboardItem>,
    #[serde(default)]
    pub groups: Vec<DashboardGroup>,
}

impl AppConfig {
    pub fn dashboard(&self) -> Dashboard {
        Dashboard {
            nodes: self.nodes.clone(),
            items: self.items.clone(),
            groups: self.groups.clone(),
        }
    }

    pub fn from_dashboard(dashboard: &Dashboard, app: &AppSettings) -> Self {
        Self {
            nodes: dashboard.nodes.clone(),
            app: app.clone(),
            items: dashboard.items.clone(),
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
    pub fn service(&self, id: &str) -> Option<&Service> {
        self.items.iter().find_map(|item| match item {
            DashboardItem::Service(service) if service.id == id => Some(service),
            _ => None,
        })
    }

    pub fn widget(&self, id: &str) -> Option<&Widget> {
        self.items.iter().find_map(|item| match item {
            DashboardItem::Widget(widget) if widget.id == id => Some(widget),
            _ => None,
        })
    }

    pub fn services(&self) -> impl Iterator<Item = &Service> {
        self.items.iter().filter_map(|item| match item {
            DashboardItem::Service(service) => Some(service),
            DashboardItem::Widget(_) => None,
        })
    }

    pub fn widgets(&self) -> impl Iterator<Item = &Widget> {
        self.items.iter().filter_map(|item| match item {
            DashboardItem::Widget(widget) => Some(widget),
            DashboardItem::Service(_) => None,
        })
    }
}
