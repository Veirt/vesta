use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use indexmap::IndexMap;
use serde::Deserialize;

use crate::config::{
    AppConfig, AppSettings, Dashboard, DashboardItem, PingConfig, Service, Widget,
};
use crate::error::{VestaError, VestaResult};

pub(crate) fn read_config(path: &Path) -> VestaResult<AppConfig> {
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(AppConfig::default());
        }
        Err(error) => return Err(error.into()),
    };
    parse_config(&source)
        .map_err(|error| VestaError::Config(format!("{}: {error}", path.display())))
}

pub(crate) fn parse_config(source: &str) -> Result<AppConfig, String> {
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
        None => AppSettings::default(),
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

pub(crate) fn write_config(
    path: &Path,
    dashboard: &Dashboard,
    app: &AppSettings,
) -> VestaResult<()> {
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
    let temp_path = path.with_file_name(format!(
        ".{file_name}.{}.tmp",
        uuid::Uuid::new_v4().simple()
    ));
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
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

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::config::{DashboardGroup, DashboardItem, Service, Widget};

    use super::{read_config, write_config};

    #[test]
    fn saves_toml_configuration_atomically_and_reads_it_back() {
        let path = std::env::temp_dir().join(format!(
            "vesta-config-{}.toml",
            uuid::Uuid::new_v4().simple()
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
        let restored = read_config(&path).unwrap();

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
    fn migrates_legacy_groups_and_attached_widgets_to_independent_tiles() {
        let path = std::env::temp_dir().join(format!(
            "vesta-legacy-config-{}.toml",
            uuid::Uuid::new_v4().simple()
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

        let mut dashboard = read_config(&path).unwrap().dashboard();
        crate::config_layout::normalize_groups(&mut dashboard);
        assert_eq!(dashboard.items.len(), 2);
        assert_eq!(dashboard.groups.len(), 1);
        assert!(matches!(dashboard.items[0], DashboardItem::Service(_)));
        assert!(matches!(dashboard.items[1], DashboardItem::Widget(_)));
        assert_eq!(dashboard.items[0].id(), "media-jellyfin");
        assert_eq!(dashboard.items[1].id(), "media-jellyfin-widget");
        let DashboardItem::Service(service) = &dashboard.items[0] else {
            unreachable!();
        };
        assert_eq!(service.group.as_deref(), Some("Media"));
        assert!(service.show_destination);
        let DashboardItem::Widget(widget) = &dashboard.items[1] else {
            unreachable!();
        };
        assert_eq!(widget.group.as_deref(), Some("Media"));
        let _ = fs::remove_file(path);
    }
}
