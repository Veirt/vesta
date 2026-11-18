use std::collections::{HashMap, HashSet};

use crate::config::{AppSettings, Dashboard, DashboardGroup, DashboardItem, Service};
use crate::config_layout::{group_span_map, item_min_size};
use crate::error::{VestaError, VestaResult};
use crate::widget_system::WidgetRegistry;

pub(crate) mod nodes;
mod primitives;

pub use primitives::{valid_http_url, valid_id, valid_image_url, valid_link};

/// Dashboard validation composed from shared primitives and node validation.
pub(crate) fn validate_dashboard_config(
    dashboard: &Dashboard,
    app: &AppSettings,
    registry: &WidgetRegistry,
) -> VestaResult<()> {
    nodes::validate_nodes(&dashboard.nodes).map_err(VestaError::InvalidDashboard)?;
    validate_groups(&dashboard.groups)?;
    validate_items(&dashboard.items, &group_span_map(dashboard), registry)?;
    if app.dashboard_title.trim().is_empty() || app.dashboard_title.chars().count() > 80 {
        return Err(VestaError::InvalidDashboard(
            "dashboard title must contain between 1 and 80 characters".to_string(),
        ));
    }
    registry.validate_dashboard(dashboard)
}

fn validate_groups(groups: &[DashboardGroup]) -> VestaResult<()> {
    let mut group_ids = HashSet::new();
    let mut group_titles = HashSet::new();
    let mut ungrouped_slot = false;
    for group in groups {
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
    Ok(())
}

fn validate_items(
    items: &[DashboardItem],
    spans: &HashMap<String, u8>,
    registry: &WidgetRegistry,
) -> VestaResult<()> {
    let mut ids = HashSet::new();
    let mut occupied = HashMap::<String, HashSet<(u16, u8)>>::new();
    for item in items {
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
        let (min_width, min_height) = item_min_size(item, spans, registry);
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
            DashboardItem::Service(service) => validate_service(id, service)?,
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
    Ok(())
}

fn validate_service(id: &str, service: &Service) -> VestaResult<()> {
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
    if let Some(ping) = &service.ping
        && !valid_http_url(&ping.url)
    {
        return Err(VestaError::InvalidDashboard(format!(
            "service '{}' availability check must be an http(s) URL",
            service.id
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::config::{DashboardGroup, DashboardItem, Service};
    use crate::config_codec::parse_config;
    use crate::metrics::node::NodeConfig;
    use crate::widget_system::WidgetRegistry;
    use crate::widgets::clock_widget::ClockWidget;
    use crate::widgets::quick_links_widget::QuickLinksWidget;
    use crate::widgets::system_stats_widget::SystemStatsWidget;
    use crate::widgets::weather_widget::WeatherWidget;

    use super::validate_dashboard_config;

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

        let error = validate_dashboard_config(&dashboard, &app, &registry).unwrap_err();
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

        assert!(validate_dashboard_config(&dashboard, &app, &registry).is_err());
        let DashboardItem::Service(service) = &mut dashboard.items[0] else {
            unreachable!();
        };
        service.href = Some("https://library.example.test".to_string());
        assert!(validate_dashboard_config(&dashboard, &app, &registry).is_ok());
        dashboard.nodes[0].endpoint = "file:///etc/passwd".to_string();
        assert!(validate_dashboard_config(&dashboard, &app, &registry).is_err());
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

        assert!(validate_dashboard_config(&dashboard, &app, &registry).is_ok());

        dashboard.groups.insert(1, ungrouped_slot(None));
        let error = validate_dashboard_config(&dashboard, &app, &registry).unwrap_err();
        assert!(error.to_string().contains("Ungrouped section"));
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
        validate_dashboard_config(&dashboard, &config.app, &registry)
            .expect("the documented configuration should be valid");
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
            validate_dashboard_config(
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
            validate_dashboard_config(
                &dashboard,
                &crate::config::AppSettings::default(),
                &registry
            )
            .is_ok()
        );
    }

    #[test]
    fn validate_rejects_malformed_widget_config_and_duplicate_groups() {
        let registry = WidgetRegistry::new().register(WeatherWidget::new(reqwest::Client::new()));
        let app = crate::config::AppSettings::default();
        let bad_lat = crate::config::Dashboard {
            nodes: vec![],
            items: vec![DashboardItem::Widget(crate::config::Widget {
                id: "weather".to_string(),
                title: "Weather".to_string(),
                name: "Weather".to_string(),
                group: None,
                config: Some(
                    [
                        ("latitude".to_string(), serde_json::json!("not-a-number")),
                        ("longitude".to_string(), serde_json::json!(10.0)),
                    ]
                    .into_iter()
                    .collect(),
                ),
                x: Some(0),
                y: Some(0),
                width: Some(4),
                height: Some(3),
            })],
            groups: vec![],
        };
        assert!(validate_dashboard_config(&bad_lat, &app, &registry).is_err());

        let duplicate_groups = crate::config::Dashboard {
            nodes: vec![],
            items: vec![],
            groups: vec![
                DashboardGroup {
                    id: "media".to_string(),
                    title: "Media".to_string(),
                    description: None,
                    icon: None,
                    width: Some(6),
                },
                DashboardGroup {
                    id: "media".to_string(),
                    title: "Movies".to_string(),
                    description: None,
                    icon: None,
                    width: Some(6),
                },
            ],
        };
        let dup_err = validate_dashboard_config(&duplicate_groups, &app, &registry).unwrap_err();
        assert!(dup_err.to_string().contains("unique"), "{dup_err}");
    }

    #[test]
    fn validate_nodes_rejects_empty_names_zero_intervals_and_bad_endpoints() {
        let valid_local = NodeConfig {
            id: "local".to_string(),
            name: "Local".to_string(),
            hostname: None,
            device_name: Some("/dev/nvme0n1p1".to_string()),
            icon: None,
            endpoint: "local".to_string(),
            interval_secs: 15,
        };
        assert!(super::nodes::validate_nodes(std::slice::from_ref(&valid_local)).is_ok());

        for bad in [
            NodeConfig {
                id: String::new(),
                ..valid_local.clone()
            },
            NodeConfig {
                name: "   ".to_string(),
                ..valid_local.clone()
            },
            NodeConfig {
                interval_secs: 0,
                ..valid_local.clone()
            },
            NodeConfig {
                endpoint: "ftp://bad.example.test/metrics".to_string(),
                ..valid_local
            },
        ] {
            assert!(super::nodes::validate_nodes(&[bad]).is_err());
        }
    }
}
