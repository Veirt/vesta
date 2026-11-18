use std::collections::{HashMap, HashSet};

use crate::config::{Dashboard, DashboardGroup, DashboardItem};
use crate::widget_system::WidgetRegistry;

/// Default column span (of 12) for groups without an explicit width; matches
/// `DEFAULT_GROUP_SPAN` in frontend layout helpers.
const DEFAULT_GROUP_SPAN: u8 = 6;
const UNKNOWN_WIDGET_MIN_SIZE: (u8, u8) = (3, 2);

/// Service links can shrink as their group grows: a tile of width `w` spans
/// `w`/12 of the group's box, so wider groups may host narrower tiles while
/// staying usable. Keep in sync with frontend `serviceMinSize`.
pub(crate) fn service_min_size(group_span: u8) -> (u8, u8) {
    match group_span {
        10..=12 => (1, 1),
        7..=9 => (2, 1),
        _ => (3, 1),
    }
}

pub(crate) fn group_span_map(dashboard: &Dashboard) -> HashMap<String, u8> {
    dashboard
        .groups
        .iter()
        .map(|group| (group.id.clone(), group.width.unwrap_or(DEFAULT_GROUP_SPAN)))
        .collect()
}

pub(crate) fn group_span(spans: &HashMap<String, u8>, group_id: &str) -> u8 {
    spans.get(group_id).copied().unwrap_or(DEFAULT_GROUP_SPAN)
}

pub(crate) fn item_min_size(
    item: &DashboardItem,
    spans: &HashMap<String, u8>,
    registry: &WidgetRegistry,
) -> (u8, u8) {
    match item {
        DashboardItem::Service(service) => {
            service_min_size(group_span(spans, service.group.as_deref().unwrap_or("")))
        }
        DashboardItem::Widget(widget) => registry
            .min_size(&widget.name)
            .unwrap_or(UNKNOWN_WIDGET_MIN_SIZE),
    }
}

pub(crate) fn normalize_layout(dashboard: &mut Dashboard, registry: &WidgetRegistry) {
    let spans = group_span_map(dashboard);
    let mut occupied = HashMap::<String, HashSet<(u16, u8)>>::new();
    for item in &mut dashboard.items {
        let group = match item {
            DashboardItem::Service(service) => service.group.as_deref().unwrap_or(""),
            DashboardItem::Widget(widget) => widget.group.as_deref().unwrap_or(""),
        };
        let occupied = occupied.entry(group.to_string()).or_default();
        let (x, y, width, height) = item.layout();
        let (default_width, default_height) = match item {
            DashboardItem::Service(_) => (4, 1),
            DashboardItem::Widget(widget) => registry.default_size(&widget.name).unwrap_or((4, 2)),
        };
        let (min_width, min_height) = item_min_size(item, &spans, registry);
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

pub(crate) fn normalize_groups(dashboard: &mut Dashboard) {
    for group in &mut dashboard.groups {
        group.width = group.width.filter(|width| (1..=12).contains(width));
    }
    let mut known = dashboard
        .groups
        .iter()
        .map(|group| group.id.clone())
        .collect::<HashSet<_>>();
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

#[cfg(test)]
mod tests {
    use crate::config::{DashboardGroup, DashboardItem, Service, Widget};
    use crate::validation::validate_dashboard_config;
    use crate::widget_system::WidgetRegistry;
    use crate::widgets::clock_widget::ClockWidget;

    use super::normalize_layout;

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
        assert!(validate_dashboard_config(&dashboard, &app, &registry).is_ok());
    }

    #[test]
    fn service_minimum_width_relaxes_as_the_group_widens() {
        let registry = WidgetRegistry::new();
        let mut dashboard = crate::config::Dashboard::default();
        for (group_id, span) in [("narrow", 3u8), ("half", 6), ("wide", 12)] {
            dashboard.groups.push(DashboardGroup {
                id: group_id.to_string(),
                title: group_id.to_string(),
                description: None,
                icon: None,
                width: Some(span),
            });
        }
        for group_id in ["narrow", "half", "wide"] {
            dashboard.items.push(DashboardItem::Service(Service {
                id: format!("tiny-{group_id}"),
                title: "Tiny".to_string(),
                group: Some(group_id.to_string()),
                icon: None,
                href: Some("https://example.test".to_string()),
                show_destination: true,
                x: Some(0),
                y: Some(0),
                width: Some(1),
                height: Some(1),
                ping: None,
            }));
        }

        normalize_layout(&mut dashboard, &registry);

        let expected: Vec<(&str, u8)> = vec![("narrow", 3), ("half", 3), ("wide", 1)];
        for (group_id, width) in expected {
            let item = dashboard
                .items
                .iter()
                .find(|item| item.id() == format!("tiny-{group_id}"))
                .unwrap();
            let (_, _, actual_width, _) = item.layout();
            assert_eq!(actual_width, Some(width), "{group_id} group");
        }
        let app = crate::config::AppSettings::default();
        assert!(validate_dashboard_config(&dashboard, &app, &registry).is_ok());
    }
}
