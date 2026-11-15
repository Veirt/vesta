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

/// Presentation settings for a dashboard section; tile identity and layout stay independent.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DashboardGroup {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Column span (of 12) the section occupies in the dashboard grid.
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

/// A dashboard tile is either a service link or a widget instance. Both have
/// their own identity and layout; neither is owned by the other.
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

/// Dashboard configuration normalized for runtime services and APIs.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Dashboard {
    #[serde(default)]
    pub nodes: Vec<crate::metrics::node::NodeConfig>,
    #[serde(default)]
    pub items: Vec<DashboardItem>,
    #[serde(default)]
    pub groups: Vec<DashboardGroup>,
}

/// One human-editable TOML document contains all dashboard and app settings.
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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    System,
    Light,
    Dark,
}

impl Default for Theme {
    fn default() -> Self {
        Self::System
    }
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

impl DashboardItem {
    pub(crate) fn default_size(&self) -> (u8, u8) {
        match self {
            Self::Service(_) => (4, 1),
            Self::Widget(widget) => match widget.name.as_str() {
                "Clock" => (4, 2),
                "QuickLinks" => (3, 2),
                "SystemStats" => (6, 4),
                "Weather" => (4, 3),
                "SonarrCalendar" => (4, 4),
                _ => (4, 2),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DashboardItem, Widget};

    #[test]
    fn built_in_widgets_have_content_appropriate_default_sizes() {
        for (name, expected) in [
            ("Clock", (4, 2)),
            ("QuickLinks", (3, 2)),
            ("SystemStats", (6, 4)),
            ("Weather", (4, 3)),
            ("SonarrCalendar", (4, 4)),
        ] {
            let item = DashboardItem::Widget(Widget {
                id: "widget".to_string(),
                title: name.to_string(),
                name: name.to_string(),
                group: None,
                config: None,
                x: None,
                y: None,
                width: None,
                height: None,
            });
            assert_eq!(item.default_size(), expected, "{name}");
        }
    }
}
