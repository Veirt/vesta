use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use indexmap::IndexMap;
use tokio::sync::{Mutex, watch};

use crate::config::{AppConfig, AppSettings, Dashboard, Widget};
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
        let config = read_config(&path)?;
        let mut dashboard = config.dashboard();
        normalize_layout(&mut dashboard);
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

    pub fn widget(&self, group: &str, title: &str) -> VestaResult<Option<Widget>> {
        Ok(self.snapshot().widget(group, title).cloned())
    }

    pub async fn save(&self, mut dashboard: Dashboard, app: AppSettings) -> VestaResult<()> {
        normalize_layout(&mut dashboard);
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

fn read_config(path: &Path) -> VestaResult<AppConfig> {
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
    normalize_layout(&mut dashboard);
    config.nodes = dashboard.nodes;
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

    let groups = match root.remove("groups") {
        Some(value) => decode_toml_value("groups", value)?,
        None => IndexMap::new(),
    };
    if let Some(key) = root.keys().next() {
        return Err(format!(
            "unexpected top-level key '{key}'; service groups must be declared under [groups.<id>]"
        ));
    }

    Ok(AppConfig { nodes, app, groups })
}

fn decode_toml_value<T: serde::de::DeserializeOwned>(
    label: &str,
    value: toml::Value,
) -> Result<T, String> {
    value
        .try_into()
        .map_err(|error| format!("invalid {label}: {error}"))
}

fn normalize_layout(dashboard: &mut Dashboard) {
    for group in dashboard.groups.values_mut() {
        for (index, service) in group.services.iter_mut().enumerate() {
            let width = service.width.unwrap_or(4).clamp(1, 12);
            let height = service
                .height
                .unwrap_or(if service.widget.is_some() { 2 } else { 1 })
                .clamp(1, 12);
            service.width = Some(width);
            service.height = Some(height);
            service.x = Some(service.x.unwrap_or(((index % 3) * 4) as u8).min(12 - width));
            service.y = Some(service.y.unwrap_or(((index / 3) * 2) as u16).min(10_000));
        }
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

    for (group_id, group) in &dashboard.groups {
        if !valid_id(group_id) || group.name.trim().is_empty() {
            return Err(VestaError::InvalidDashboard(
                "service group IDs must use letters, numbers, hyphens, or underscores, and names are required".to_string(),
            ));
        }
        let mut titles = std::collections::HashSet::new();
        for service in &group.services {
            if service.title.trim().is_empty() {
                return Err(VestaError::InvalidDashboard(format!(
                    "service in group '{group_id}' needs a title"
                )));
            }
            if !titles.insert(&service.title) {
                return Err(VestaError::InvalidDashboard(format!(
                    "service title '{}' is duplicated in group '{group_id}'",
                    service.title
                )));
            }
            if service.title.chars().count() > 100 {
                return Err(VestaError::InvalidDashboard(format!(
                    "service title in group '{group_id}' must be no longer than 100 characters"
                )));
            }
            if service
                .width
                .is_some_and(|width| !(1..=12).contains(&width))
                || service
                    .height
                    .is_some_and(|height| !(1..=12).contains(&height))
                || service.x.is_some_and(|x| x > 11)
                || service.y.is_some_and(|y| y > 10_000)
            {
                return Err(VestaError::InvalidDashboard(format!(
                    "service '{}' has an invalid saved layout",
                    service.title
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
                        service.title
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

    use indexmap::IndexMap;

    use crate::config::{Group, Service};
    use crate::metrics::node::NodeConfig;
    use crate::widget_system::WidgetRegistry;
    use crate::widgets::clock_widget::ClockWidget;
    use crate::widgets::quick_links_widget::QuickLinksWidget;
    use crate::widgets::system_stats_widget::SystemStatsWidget;
    use crate::widgets::weather_widget::WeatherWidget;

    use super::{normalize_layout, parse_config, read_config, validate, write_config};

    #[test]
    fn rejects_unsafe_urls_and_accepts_http_sources() {
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
        dashboard.groups = IndexMap::from([(
            "media".to_string(),
            Group {
                name: "Media".to_string(),
                services: vec![Service {
                    title: "Library".to_string(),
                    icon: None,
                    href: Some("javascript:alert(1)".to_string()),
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                    widget: None,
                    ping: None,
                }],
            },
        )]);

        assert!(validate(&dashboard, &app, &registry).is_err());
        dashboard.groups["media"].services[0].href =
            Some("https://library.example.test".to_string());
        assert!(validate(&dashboard, &app, &registry).is_ok());
        dashboard.nodes[0].endpoint = "file:///etc/passwd".to_string();
        assert!(validate(&dashboard, &app, &registry).is_err());
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
        let mut dashboard = crate::config::Dashboard::default();
        dashboard.groups.insert(
            "media".to_string(),
            Group {
                name: "Media".to_string(),
                services: vec![Service {
                    title: "Jellyfin".to_string(),
                    icon: Some("/api/icons/12".to_string()),
                    href: Some("https://media.example.test".to_string()),
                    x: Some(4),
                    y: Some(2),
                    width: Some(4),
                    height: Some(2),
                    widget: None,
                    ping: None,
                }],
            },
        );
        dashboard.groups.insert(
            "app".to_string(),
            Group {
                name: "Application services".to_string(),
                services: Vec::new(),
            },
        );

        write_config(&path, &dashboard, &app).unwrap();
        let restored = read_config(&path).unwrap();

        assert_eq!(restored.app.dashboard_title, "Lab");
        assert_eq!(
            restored.groups["media"].services[0].icon.as_deref(),
            Some("/api/icons/12")
        );
        assert_eq!(restored.groups["media"].services[0].x, Some(4));
        assert_eq!(restored.groups["media"].services[0].width, Some(4));
        assert_eq!(restored.groups["app"].name, "Application services");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn documented_toml_example_loads_and_validates() {
        let config = parse_config(include_str!("../docs/vesta.toml"))
            .expect("the documented configuration should parse");
        let mut dashboard = config.dashboard();
        normalize_layout(&mut dashboard);
        let registry = WidgetRegistry::new()
            .register(ClockWidget::new())
            .register(QuickLinksWidget::new())
            .register(SystemStatsWidget::new())
            .register(WeatherWidget::new(reqwest::Client::new()));
        validate(&dashboard, &config.app, &registry)
            .expect("the documented configuration should be valid");
    }

    #[test]
    fn rejects_service_groups_outside_groups_namespace() {
        let path = std::env::temp_dir().join(format!(
            "vesta-unscoped-config-{}-{}.toml",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::write(
            &path,
            r#"
[media]
name = "Media"
"#,
        )
        .unwrap();

        let error = read_config(&path).unwrap_err().to_string();
        assert!(error.contains("service groups must be declared under [groups.<id>]"));
        let _ = fs::remove_file(path);
    }
}
