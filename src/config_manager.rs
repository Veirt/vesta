use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use indexmap::IndexMap;
use serde::Deserialize;
use tokio::sync::{Mutex, watch};

use crate::config::{
    AppConfig, AppSettings, Dashboard, DashboardGroup, DashboardItem, PingConfig, Service, Widget,
};
use crate::error::{VestaError, VestaResult};
use crate::metrics::service::validate_nodes;
use crate::widget_system::WidgetRegistry;

pub struct SettingsSnapshot {
    pub dashboard: Arc<Dashboard>,
    pub app: AppSettings,
}

/// Validates, writes, and atomically publishes the active TOML configuration.
pub struct ConfigManager {
    path: PathBuf,
    snapshot: RwLock<Arc<SettingsSnapshot>>,
    write_gate: Mutex<()>,
    registry: Arc<WidgetRegistry>,
    updates: watch::Sender<u64>,
}

impl ConfigManager {
    pub fn new(path: impl Into<PathBuf>, registry: Arc<WidgetRegistry>) -> VestaResult<Self> {
        let path = path.into();
        let config = read_config(&path, &registry)?;
        let mut dashboard = config.dashboard();
        normalize_layout(&mut dashboard, &registry);
        normalize_groups(&mut dashboard);
        let app = config.app;
        validate(&dashboard, &app, &registry)?;
        let (updates, _) = watch::channel(0);

        Ok(Self {
            path,
            snapshot: RwLock::new(Arc::new(SettingsSnapshot {
                dashboard: Arc::new(dashboard),
                app,
            })),
            write_gate: Mutex::new(()),
            registry,
            updates,
        })
    }

    pub fn snapshot(&self) -> Arc<Dashboard> {
        self.snapshot
            .read()
            .expect("settings snapshot lock poisoned")
            .dashboard
            .clone()
    }

    pub fn settings_snapshot(&self) -> Arc<SettingsSnapshot> {
        self.snapshot
            .read()
            .expect("settings snapshot lock poisoned")
            .clone()
    }

    pub fn subscribe_updates(&self) -> watch::Receiver<u64> {
        self.updates.subscribe()
    }

    pub fn widget(&self, id: &str) -> VestaResult<Option<Widget>> {
        Ok(self.snapshot().widget(id).cloned())
    }

    pub async fn save(&self, mut dashboard: Dashboard, app: AppSettings) -> VestaResult<()> {
        normalize_layout(&mut dashboard, &self.registry);
        normalize_groups(&mut dashboard);
        validate(&dashboard, &app, &self.registry)?;
        let _write = self.write_gate.lock().await;
        write_config(&self.path, &dashboard, &app)?;

        *self
            .snapshot
            .write()
            .expect("settings snapshot lock poisoned") = Arc::new(SettingsSnapshot {
            dashboard: Arc::new(dashboard),
            app,
        });
        self.updates
            .send_modify(|revision| *revision = revision.wrapping_add(1));
        Ok(())
    }
}

fn read_config(path: &Path, registry: &WidgetRegistry) -> VestaResult<AppConfig> {
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(AppConfig::default());
        }
        Err(error) => return Err(error.into()),
    };
    let mut config = parse_config(&source)
        .map_err(|error| VestaError::Config(format!("{}: {error}", path.display())))?;
    let mut dashboard = config.dashboard();
    normalize_layout(&mut dashboard, registry);
    normalize_groups(&mut dashboard);
    config.nodes = dashboard.nodes;
    config.items = dashboard.items;
    config.groups = dashboard.groups;
    Ok(config)
}

fn parse_config(source: &str) -> Result<AppConfig, String> {
    let value: toml::Value = toml::from_str(source).map_err(|error| error.to_string())?;
    let toml::Value::Table(mut root) = value else {
        return Err("the configuration root must be a TOML table".to_string());
    };

    let nodes = match root.remove("nodes") {
        Some(value) => decode_toml_value("nodes", value)?,
        None => Vec::new(),
    };
    let app = match root.remove("app") {
        Some(value) => decode_toml_value("app settings", value)?,
        _ => AppSettings::default(),
    };

    let (items, groups) = match (root.remove("items"), root.remove("groups")) {
        (Some(items), Some(toml::Value::Array(groups))) => (
            decode_toml_value("dashboard items", items)?,
            decode_toml_value("dashboard groups", toml::Value::Array(groups))?,
        ),
        (Some(_), Some(_)) => {
            return Err(
                "use [[items]] and [[groups]] instead of the legacy [groups] layout".to_string(),
            );
        }
        (Some(value), None) => (decode_toml_value("dashboard items", value)?, Vec::new()),
        (None, Some(toml::Value::Array(groups))) => (
            Vec::new(),
            decode_toml_value("dashboard groups", toml::Value::Array(groups))?,
        ),
        (None, Some(value)) => (
            migrate_legacy_groups(decode_toml_value("groups", value)?),
            Vec::new(),
        ),
        (None, None) => (Vec::new(), Vec::new()),
    };
    if let Some(key) = root.keys().next() {
        return Err(format!(
            "unexpected top-level key '{key}'; dashboard tiles must be declared under [[items]]"
        ));
    }

    Ok(AppConfig {
        nodes,
        app,
        items,
        groups,
    })
}

#[derive(Deserialize)]
struct LegacyGroup {
    name: String,
    #[serde(default)]
    services: Vec<LegacyService>,
}

#[derive(Deserialize)]
struct LegacyService {
    title: String,
    #[serde(default)]
    icon: Option<String>,
    #[serde(default)]
    href: Option<String>,
    #[serde(default)]
    width: Option<u8>,
    #[serde(default)]
    height: Option<u8>,
    #[serde(default)]
    widget: Option<LegacyWidget>,
    #[serde(default)]
    ping: Option<PingConfig>,
}

#[derive(Deserialize)]
struct LegacyWidget {
    name: String,
    #[serde(default)]
    config: Option<crate::config::WidgetConfig>,
}

fn migrate_legacy_groups(groups: IndexMap<String, LegacyGroup>) -> Vec<DashboardItem> {
    let mut used_ids = std::collections::HashSet::new();
    let mut items = Vec::new();

    for (group_id, group) in groups {
        let group_name = group.name.trim().to_string();
        let group_name = if group_name.is_empty() {
            group_id.clone()
        } else {
            group_name
        };
        for service in group.services {
            let service_id =
                unique_migration_id(&format!("{group_id}-{}", service.title), &used_ids);
            used_ids.insert(service_id.clone());
            let legacy_widget = service.widget;
            items.push(DashboardItem::Service(Service {
                id: service_id.clone(),
                title: service.title,
                group: Some(group_name.clone()),
                icon: service.icon,
                href: service.href,
                show_destination: true,
                x: None,
                y: None,
                width: Some(if legacy_widget.is_some() {
                    4
                } else {
                    service.width.unwrap_or(4)
                }),
                height: Some(if legacy_widget.is_some() {
                    1
                } else {
                    service.height.unwrap_or(1)
                }),
                ping: service.ping,
            }));

            if let Some(widget) = legacy_widget {
                let widget_id = unique_migration_id(&format!("{service_id}-widget"), &used_ids);
                used_ids.insert(widget_id.clone());
                items.push(DashboardItem::Widget(Widget {
                    id: widget_id,
                    title: widget.name.clone(),
                    name: widget.name,
                    group: Some(group_name.clone()),
                    config: widget.config,
                    x: None,
                    y: None,
                    width: service.width.or(Some(4)),
                    height: service.height.or(Some(2)),
                }));
            }
        }
    }
    items
}

fn unique_migration_id(value: &str, used: &std::collections::HashSet<String>) -> String {
    let base = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .chars()
        .take(54)
        .collect::<String>();
    let base = if base.is_empty() { "tile" } else { &base };
    let mut candidate = base.to_string();
    let mut suffix = 2;
    while used.contains(&candidate) {
        candidate = format!("{base}-{suffix}");
        suffix += 1;
    }
    candidate
}

fn decode_toml_value<T: serde::de::DeserializeOwned>(
    label: &str,
    value: toml::Value,
) -> Result<T, String> {
    value
        .try_into()
        .map_err(|error| format!("invalid {label}: {error}"))
}

const SERVICE_MIN_SIZE: (u8, u8) = (3, 1);

fn item_min_size(item: &DashboardItem, registry: &WidgetRegistry) -> (u8, u8) {
    match item {
        DashboardItem::Service(_) => SERVICE_MIN_SIZE,
        DashboardItem::Widget(widget) => registry.min_size(&widget.name).unwrap_or((3, 2)),
    }
}

fn normalize_layout(dashboard: &mut Dashboard, registry: &WidgetRegistry) {
    let mut occupied = std::collections::HashMap::<String, std::collections::HashSet<_>>::new();
    for item in &mut dashboard.items {
        let group = match item {
            DashboardItem::Service(service) => service.group.as_deref().unwrap_or(""),
            DashboardItem::Widget(widget) => widget.group.as_deref().unwrap_or(""),
        };
        let occupied = occupied.entry(group.to_string()).or_default();
        let (x, y, width, height) = item.layout();
        let (default_width, default_height) = item.default_size();
        let (min_width, min_height) = item_min_size(item, registry);
        let width = width.unwrap_or(default_width).clamp(min_width, 12);
        let height = height.unwrap_or(default_height).clamp(min_height, 12);
        let (x, y) = match (x, y) {
            (Some(x), Some(y)) => (x.min(12 - width), y.min(10_000)),
            _ => {
                let mut placed = None;
                'rows: for y in 0..=10_000 {
                    for x in 0..=12 - width {
                        if (y..y + height as u16).all(|row| {
                            (x..x + width).all(|column| !occupied.contains(&(row, column)))
                        }) {
                            placed = Some((x, y));
                            break 'rows;
                        }
                    }
                }
                placed.unwrap_or((0, 10_000))
            }
        };
        item.set_layout(x, y, width, height);
        for row in y..y + height as u16 {
            for column in x..x + width {
                occupied.insert((row, column));
            }
        }
    }
}

fn normalize_groups(dashboard: &mut Dashboard) {
    for group in &mut dashboard.groups {
        group.width = group.width.filter(|width| (1..=12).contains(width));
    }
    let mut known = dashboard
        .groups
        .iter()
        .map(|group| group.id.clone())
        .collect::<std::collections::HashSet<_>>();
    for item in &dashboard.items {
        let id = match item {
            DashboardItem::Service(service) => service.group.as_deref(),
            DashboardItem::Widget(widget) => widget.group.as_deref(),
        };
        let Some(id) = id.filter(|id| known.insert((*id).to_string())) else {
            continue;
        };
        dashboard.groups.push(DashboardGroup {
            id: id.to_string(),
            title: id.to_string(),
            description: None,
            icon: None,
            width: None,
        });
    }
}

fn write_config(path: &Path, dashboard: &Dashboard, app: &AppSettings) -> VestaResult<()> {
    let source = toml::to_string_pretty(&AppConfig::from_dashboard(dashboard, app))
        .map_err(|error| VestaError::Config(format!("could not serialize TOML: {error}")))?;
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("config.toml");
    let temp_path = path.with_file_name(format!(".{file_name}.{}.tmp", std::process::id()));
    let mut options = OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let write_result = (|| -> VestaResult<()> {
        let mut file = options.open(&temp_path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(fs::Permissions::from_mode(0o600))?;
        }
        file.write_all(source.as_bytes())?;
        file.sync_all()?;
        fs::rename(&temp_path, path)?;
        Ok(())
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    write_result
}

fn validate(
    dashboard: &Dashboard,
    app: &AppSettings,
    registry: &WidgetRegistry,
) -> VestaResult<()> {
    validate_nodes(&dashboard.nodes)
        .map_err(|error| VestaError::InvalidDashboard(error.to_string()))?;
    for node in &dashboard.nodes {
        if node.id.trim().is_empty()
            || node.name.trim().is_empty()
            || node.endpoint.trim().is_empty()
        {
            return Err(VestaError::InvalidDashboard(
                "node ID, name, and endpoint are required".to_string(),
            ));
        }
        if !(1..=86_400).contains(&node.interval_secs) {
            return Err(VestaError::InvalidDashboard(format!(
                "node '{}' scrape interval must be between 1 and 86400 seconds",
                node.id
            )));
        }
        if !valid_id(&node.id) {
            return Err(VestaError::InvalidDashboard(format!(
                "node ID '{}' must use letters, numbers, hyphens, or underscores",
                node.id
            )));
        }
        if let Some(icon) = node.icon.as_deref().filter(|icon| !icon.trim().is_empty())
            && !valid_image_url(icon)
        {
            return Err(VestaError::InvalidDashboard(format!(
                "server '{}' icon must be an http(s) URL or an uploaded icon path",
                node.id
            )));
        }
        if !node.is_local() {
            let endpoint = reqwest::Url::parse(&node.endpoint).map_err(|_| {
                VestaError::InvalidDashboard(format!(
                    "node '{}' endpoint must be `local` or a valid http(s) URL",
                    node.id
                ))
            })?;
            if !matches!(endpoint.scheme(), "http" | "https") || endpoint.host_str().is_none() {
                return Err(VestaError::InvalidDashboard(format!(
                    "node '{}' endpoint must use http or https",
                    node.id
                )));
            }
        }
    }

    let mut group_ids = std::collections::HashSet::new();
    let mut group_titles = std::collections::HashSet::new();
    let mut ungrouped_slot = false;
    for group in &dashboard.groups {
        // An id-less entry carries the implicit Ungrouped section's order and
        // width; tiles never reference it because ungrouped means `group: null`.
        if group.id.is_empty() {
            if ungrouped_slot {
                return Err(VestaError::InvalidDashboard(
                    "at most one Ungrouped section entry is allowed".to_string(),
                ));
            }
            ungrouped_slot = true;
        } else {
            if group.id.trim().is_empty()
                || group.id.trim() != group.id
                || group.id.chars().count() > 64
                || !group_ids.insert(group.id.as_str())
            {
                return Err(VestaError::InvalidDashboard(
                    "dashboard group IDs must be unique and contain 1 to 64 characters".to_string(),
                ));
            }
            let normalized_title = group.title.trim().to_lowercase();
            if normalized_title.is_empty()
                || group.title.chars().count() > 64
                || !group_titles.insert(normalized_title)
            {
                return Err(VestaError::InvalidDashboard(format!(
                    "dashboard group '{}' needs a unique title of 1 to 64 characters",
                    group.id
                )));
            }
        }
        if group
            .description
            .as_deref()
            .is_some_and(|description| description.chars().count() > 240)
        {
            return Err(VestaError::InvalidDashboard(format!(
                "dashboard group '{}' description must be 240 characters or fewer",
                group.id
            )));
        }
        if let Some(icon) = group.icon.as_deref().filter(|icon| !icon.trim().is_empty())
            && !valid_image_url(icon)
        {
            return Err(VestaError::InvalidDashboard(format!(
                "dashboard group '{}' icon must be an http(s) URL or an uploaded icon path",
                group.id
            )));
        }
        if let Some(width) = group.width
            && !(1..=12).contains(&width)
        {
            return Err(VestaError::InvalidDashboard(format!(
                "dashboard group '{}' width must be between 1 and 12",
                group.id
            )));
        }
    }

    let mut ids = std::collections::HashSet::new();
    let mut occupied = std::collections::HashMap::<String, std::collections::HashSet<_>>::new();
    for item in &dashboard.items {
        let id = item.id();
        if !valid_id(id) || !ids.insert(id) {
            return Err(VestaError::InvalidDashboard(format!(
                "dashboard tile ID '{id}' must be unique and use letters, numbers, hyphens, or underscores"
            )));
        }
        let (x, y, width, height) = item.layout();
        let (x, y, width, height) = (
            x.unwrap_or(0),
            y.unwrap_or(0),
            width.unwrap_or(1),
            height.unwrap_or(1),
        );
        if !(1..=12).contains(&width) || !(1..=12).contains(&height) || x + width > 12 || y > 10_000
        {
            return Err(VestaError::InvalidDashboard(format!(
                "dashboard tile '{id}' has an invalid saved layout"
            )));
        }
        let (min_width, min_height) = item_min_size(item, registry);
        if width < min_width || height < min_height {
            return Err(VestaError::InvalidDashboard(format!(
                "dashboard tile '{id}' is smaller than its minimum size of {min_width}×{min_height}"
            )));
        }
        let group = match item {
            DashboardItem::Service(service) => service.group.as_deref().unwrap_or(""),
            DashboardItem::Widget(widget) => widget.group.as_deref().unwrap_or(""),
        };
        let group_occupied = occupied.entry(group.to_string()).or_default();
        for row in y..y + height as u16 {
            for column in x..x + width {
                if !group_occupied.insert((row, column)) {
                    return Err(VestaError::InvalidDashboard(
                        "tiles in the same group cannot overlap".to_string(),
                    ));
                }
            }
        }

        let item_group = match item {
            DashboardItem::Service(service) => service.group.as_deref(),
            DashboardItem::Widget(widget) => widget.group.as_deref(),
        };
        if let Some(group) = item_group
            && (group.trim().is_empty() || group.trim() != group || group.chars().count() > 64)
        {
            return Err(VestaError::InvalidDashboard(format!(
                "dashboard tile '{id}' group must contain 1 to 64 characters"
            )));
        }

        match item {
            DashboardItem::Service(service) => {
                if service.title.trim().is_empty() {
                    return Err(VestaError::InvalidDashboard(format!(
                        "service '{id}' needs a title"
                    )));
                }
                if service.title.chars().count() > 100 {
                    return Err(VestaError::InvalidDashboard(format!(
                        "service '{}' title must be no longer than 100 characters",
                        service.id
                    )));
                }
                if let Some(icon) = service
                    .icon
                    .as_deref()
                    .filter(|icon| !icon.trim().is_empty())
                    && !valid_image_url(icon)
                {
                    return Err(VestaError::InvalidDashboard(format!(
                        "service '{}' icon must be an http(s) URL or an uploaded icon path",
                        service.title
                    )));
                }
                if let Some(href) = service
                    .href
                    .as_deref()
                    .filter(|href| !href.trim().is_empty())
                    && !valid_link(href)
                {
                    return Err(VestaError::InvalidDashboard(format!(
                        "service '{}' link must be a relative path or http(s) URL",
                        service.title
                    )));
                }
                if let Some(ping) = &service.ping {
                    let valid = reqwest::Url::parse(&ping.url)
                        .ok()
                        .is_some_and(|url| matches!(url.scheme(), "http" | "https"));
                    if !valid {
                        return Err(VestaError::InvalidDashboard(format!(
                            "service '{}' availability check must be an http(s) URL",
                            service.id
                        )));
                    }
                }
            }
            DashboardItem::Widget(widget) => {
                if widget.title.trim().is_empty() || widget.title.chars().count() > 100 {
                    return Err(VestaError::InvalidDashboard(format!(
                        "widget '{}' title must contain between 1 and 100 characters",
                        widget.id
                    )));
                }
            }
        }
    }
    if app.dashboard_title.trim().is_empty() || app.dashboard_title.chars().count() > 80 {
        return Err(VestaError::InvalidDashboard(
            "dashboard title must contain between 1 and 80 characters".to_string(),
        ));
    }
    registry.validate_dashboard(dashboard)
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

pub fn valid_link(value: &str) -> bool {
    if value.starts_with('/') && !value.starts_with("//") {
        return true;
    }
    reqwest::Url::parse(value)
        .ok()
        .is_some_and(|url| matches!(url.scheme(), "http" | "https"))
}

pub fn valid_image_url(value: &str) -> bool {
    if value.starts_with("/api/icons/") {
        return value
            .strip_prefix("/api/icons/")
            .is_some_and(|id| !id.is_empty() && id.bytes().all(|byte| byte.is_ascii_digit()));
    }
    reqwest::Url::parse(value)
        .ok()
        .is_some_and(|url| matches!(url.scheme(), "http" | "https"))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::config::{DashboardItem, Service, Widget};
    use crate::metrics::node::NodeConfig;
    use crate::widget_system::WidgetRegistry;
    use crate::widgets::clock_widget::ClockWidget;
    use crate::widgets::quick_links_widget::QuickLinksWidget;
    use crate::widgets::system_stats_widget::SystemStatsWidget;
    use crate::widgets::weather_widget::WeatherWidget;

    use super::{
        DashboardGroup, normalize_layout, parse_config, read_config, validate, write_config,
    };

    #[test]
    fn undersized_tiles_are_clamped_to_their_minimum_size_on_load() {
        let registry = WidgetRegistry::new().register(ClockWidget::new());
        let mut dashboard = crate::config::Dashboard::default();
        dashboard.items.push(DashboardItem::Service(Service {
            id: "tiny".to_string(),
            title: "Tiny".to_string(),
            group: None,
            icon: None,
            href: Some("https://example.test".to_string()),
            show_destination: true,
            x: Some(0),
            y: Some(0),
            width: Some(1),
            height: Some(1),
            ping: None,
        }));
        dashboard.items.push(DashboardItem::Widget(Widget {
            id: "tiny-clock".to_string(),
            title: "Clock".to_string(),
            name: "Clock".to_string(),
            group: None,
            config: None,
            x: Some(3),
            y: Some(0),
            width: Some(6),
            height: Some(1),
        }));

        normalize_layout(&mut dashboard, &registry);

        let (x, y, width, height) = dashboard.items[0].layout();
        assert_eq!((x, y, width, height), (Some(0), Some(0), Some(3), Some(1)));
        let (x, y, width, height) = dashboard.items[1].layout();
        assert_eq!((x, y, width, height), (Some(3), Some(0), Some(6), Some(2)));
        let app = crate::config::AppSettings::default();
        assert!(validate(&dashboard, &app, &registry).is_ok());
    }

    #[test]
    fn validate_rejects_tiles_below_their_minimum_size() {
        let registry = WidgetRegistry::new().register(ClockWidget::new());
        let app = crate::config::AppSettings::default();
        let mut dashboard = crate::config::Dashboard::default();
        dashboard.items.push(DashboardItem::Service(Service {
            id: "tiny".to_string(),
            title: "Tiny".to_string(),
            group: None,
            icon: None,
            href: Some("https://example.test".to_string()),
            show_destination: true,
            x: Some(0),
            y: Some(0),
            width: Some(1),
            height: Some(1),
            ping: None,
        }));

        let error = validate(&dashboard, &app, &registry).unwrap_err();
        assert!(error.to_string().contains("minimum size"));
    }

    #[test]
    fn validates_independent_services_and_http_sources() {
        let registry = WidgetRegistry::new();
        let app = crate::config::AppSettings::default();
        let mut dashboard = crate::config::Dashboard::default();
        dashboard.nodes.push(NodeConfig {
            id: "nas".to_string(),
            name: "NAS".to_string(),
            hostname: None,
            device_name: None,
            icon: None,
            endpoint: "https://nas.example.test/metrics".to_string(),
            interval_secs: 15,
        });
        dashboard.items.push(DashboardItem::Service(Service {
            id: "library".to_string(),
            title: "Library".to_string(),
            group: None,
            icon: None,
            href: Some("javascript:alert(1)".to_string()),
            show_destination: true,
            x: Some(0),
            y: Some(0),
            width: Some(4),
            height: Some(1),
            ping: None,
        }));

        assert!(validate(&dashboard, &app, &registry).is_err());
        let DashboardItem::Service(service) = &mut dashboard.items[0] else {
            unreachable!();
        };
        service.href = Some("https://library.example.test".to_string());
        assert!(validate(&dashboard, &app, &registry).is_ok());
        dashboard.nodes[0].endpoint = "file:///etc/passwd".to_string();
        assert!(validate(&dashboard, &app, &registry).is_err());
    }

    fn ungrouped_slot(width: Option<u8>) -> DashboardGroup {
        DashboardGroup {
            id: String::new(),
            title: String::new(),
            description: None,
            icon: None,
            width,
        }
    }

    #[test]
    fn validate_accepts_a_single_ungrouped_slot_entry() {
        let registry = WidgetRegistry::new().register(ClockWidget::new());
        let app = crate::config::AppSettings::default();
        let mut dashboard = crate::config::Dashboard::default();
        dashboard.groups.push(DashboardGroup {
            id: "media".to_string(),
            title: "Media".to_string(),
            description: None,
            icon: None,
            width: None,
        });
        dashboard.groups.push(ungrouped_slot(Some(6)));
        dashboard.groups.push(DashboardGroup {
            id: "docs".to_string(),
            title: "Docs".to_string(),
            description: None,
            icon: None,
            width: None,
        });

        assert!(validate(&dashboard, &app, &registry).is_ok());

        dashboard.groups.insert(1, ungrouped_slot(None));
        let error = validate(&dashboard, &app, &registry).unwrap_err();
        assert!(error.to_string().contains("Ungrouped section"));
    }

    #[test]
    fn saves_toml_configuration_atomically_and_reads_it_back() {
        let path = std::env::temp_dir().join(format!(
            "vesta-config-{}-{}.toml",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let app = crate::config::AppSettings {
            dashboard_title: "Lab".to_string(),
            theme: crate::config::Theme::Dark,
        };
        let dashboard = crate::config::Dashboard {
            nodes: vec![],
            items: vec![
                DashboardItem::Service(Service {
                    id: "jellyfin".to_string(),
                    title: "Jellyfin".to_string(),
                    group: Some("Media".to_string()),
                    icon: Some("/api/icons/12".to_string()),
                    href: Some("https://media.example.test".to_string()),
                    show_destination: false,
                    x: Some(4),
                    y: Some(2),
                    width: Some(4),
                    height: Some(1),
                    ping: None,
                }),
                DashboardItem::Widget(Widget {
                    id: "clock".to_string(),
                    title: "Local time".to_string(),
                    name: "Clock".to_string(),
                    group: Some("Media".to_string()),
                    config: None,
                    x: Some(0),
                    y: Some(0),
                    width: Some(4),
                    height: Some(2),
                }),
            ],
            groups: vec![DashboardGroup {
                id: "Media".to_string(),
                title: "Entertainment".to_string(),
                description: Some("Streaming services".to_string()),
                icon: None,
                width: None,
            }],
        };

        write_config(&path, &dashboard, &app).unwrap();
        let restored = read_config(&path, &WidgetRegistry::new()).unwrap();

        assert_eq!(restored.app.dashboard_title, "Lab");
        assert_eq!(restored.items.len(), 2);
        let DashboardItem::Service(service) = &restored.items[0] else {
            panic!("first tile should be a service");
        };
        assert_eq!(service.id, "jellyfin");
        assert_eq!(service.icon.as_deref(), Some("/api/icons/12"));
        assert!(!service.show_destination);
        assert_eq!(service.x, Some(4));
        assert_eq!(restored.groups[0].title, "Entertainment");
        let DashboardItem::Widget(widget) = &restored.items[1] else {
            panic!("second tile should be a widget");
        };
        assert_eq!(widget.title, "Local time");
        assert_eq!(widget.name, "Clock");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn documented_toml_example_loads_and_validates() {
        let config = parse_config(include_str!("../docs/vesta.toml"))
            .expect("the documented configuration should parse");
        assert_eq!(config.groups.len(), 2);
        let dashboard = config.dashboard();
        let registry = WidgetRegistry::new()
            .register(ClockWidget::new())
            .register(QuickLinksWidget::new())
            .register(SystemStatsWidget::new())
            .register(WeatherWidget::new(reqwest::Client::new()));
        validate(&dashboard, &config.app, &registry)
            .expect("the documented configuration should be valid");
    }

    #[test]
    fn migrates_legacy_groups_and_attached_widgets_to_independent_tiles() {
        let path = std::env::temp_dir().join(format!(
            "vesta-legacy-config-{}-{}.toml",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::write(
            &path,
            r#"
[groups.media]
name = "Media"

[[groups.media.services]]
title = "Jellyfin"
href = "https://media.example.test"
x = 0
y = 0
width = 6
height = 2
widget.name = "Clock"
"#,
        )
        .unwrap();

        let config = read_config(&path, &WidgetRegistry::new()).unwrap();
        assert_eq!(config.items.len(), 2);
        assert_eq!(config.groups.len(), 1);
        assert!(matches!(config.items[0], DashboardItem::Service(_)));
        assert!(matches!(config.items[1], DashboardItem::Widget(_)));
        assert_eq!(config.items[0].id(), "media-jellyfin");
        assert_eq!(config.items[1].id(), "media-jellyfin-widget");
        let DashboardItem::Service(service) = &config.items[0] else {
            unreachable!();
        };
        assert_eq!(service.group.as_deref(), Some("Media"));
        assert!(service.show_destination);
        let DashboardItem::Widget(widget) = &config.items[1] else {
            unreachable!();
        };
        assert_eq!(widget.group.as_deref(), Some("Media"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_overlapping_dashboard_tiles() {
        let registry = WidgetRegistry::new();
        let dashboard = crate::config::Dashboard {
            nodes: vec![],
            items: vec![
                DashboardItem::Service(Service {
                    id: "first".to_string(),
                    title: "First".to_string(),
                    group: None,
                    icon: None,
                    href: None,
                    show_destination: true,
                    x: Some(0),
                    y: Some(0),
                    width: Some(4),
                    height: Some(2),
                    ping: None,
                }),
                DashboardItem::Service(Service {
                    id: "second".to_string(),
                    title: "Second".to_string(),
                    group: None,
                    icon: None,
                    href: None,
                    show_destination: true,
                    x: Some(3),
                    y: Some(1),
                    width: Some(4),
                    height: Some(1),
                    ping: None,
                }),
            ],
            groups: vec![],
        };
        assert!(
            validate(
                &dashboard,
                &crate::config::AppSettings::default(),
                &registry
            )
            .is_err()
        );
    }

    #[test]
    fn permits_layouts_to_overlap_between_distinct_groups() {
        let registry = WidgetRegistry::new();
        let dashboard = crate::config::Dashboard {
            nodes: vec![],
            items: vec![
                DashboardItem::Service(Service {
                    id: "first".to_string(),
                    title: "First".to_string(),
                    group: Some("Media".to_string()),
                    icon: None,
                    href: None,
                    show_destination: true,
                    x: Some(0),
                    y: Some(0),
                    width: Some(4),
                    height: Some(2),
                    ping: None,
                }),
                DashboardItem::Service(Service {
                    id: "second".to_string(),
                    title: "Second".to_string(),
                    group: Some("System".to_string()),
                    icon: None,
                    href: None,
                    show_destination: true,
                    x: Some(0),
                    y: Some(0),
                    width: Some(4),
                    height: Some(2),
                    ping: None,
                }),
            ],
            groups: vec![],
        };
        assert!(
            validate(
                &dashboard,
                &crate::config::AppSettings::default(),
                &registry
            )
            .is_ok()
        );
    }
}
