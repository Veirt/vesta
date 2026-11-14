use std::sync::Arc;

use axum::Extension;
use maud::{DOCTYPE, Markup, html};

use crate::config::{Dashboard, Group, Service};
use crate::ping::service_indicator;
use crate::widget_system::{WidgetRef, service_layout_style};
use crate::{AppState, widget_system::WidgetRegistry};

fn service_card_body(group_id: &str, service: &Service) -> Markup {
    let icon_src = service.icon.as_deref().filter(|src| !src.trim().is_empty());
    html! {
        @if let Some(icon_src) = icon_src {
            img class="service-icon" src=(icon_src) alt="" loading="lazy" decoding="async";
        } @else {
            span class="service-icon-fallback" aria-hidden="true" {
                (service.title.chars().next().unwrap_or('S').to_uppercase())
            }
        }
        span class="service-copy" {
            span class="service-title" { (service.title) }
            span class="service-subtitle" { (group_id) }
        }
        @if service.ping.is_some() {
            (service_indicator(group_id, &service.title))
        } @else {
            span class="service-open-mark" aria-hidden="true" { "↗" }
        }
    }
}

fn service_card(group_id: &str, service: &Service) -> Markup {
    let search_text = format!("{} {}", service.title, group_id);
    html! {
        @if let Some(href) = service.href.as_deref().filter(|href| !href.is_empty()) {
            a class="service-card service-layout-item" style=(service_layout_style(service))
                href=(href) target="_blank" rel="noreferrer"
                data-search-item data-service-card data-search-text=(search_text) {
                (service_card_body(group_id, service))
            }
        } @else {
            div class="service-card service-layout-item" style=(service_layout_style(service))
                data-search-item data-service-card data-search-text=(search_text) {
                (service_card_body(group_id, service))
            }
        }
    }
}

fn group_grid(group_id: &str, group: &Group, widget_registry: &WidgetRegistry) -> Markup {
    html! {
        div class="service-grid" {
            @for service in &group.services {
                @if let Some(widget) = &service.widget {
                    div class="widget-search-item" data-search-item
                        data-search-text=(format!("{} {}", service.title, group_id)) {
                        (widget_registry.render_widget(
                            &WidgetRef::new(group_id, &service.title),
                            service,
                            widget,
                        ))
                    }
                } @else {
                    (service_card(group_id, service))
                }
            }
        }
    }
}

fn group_section(group_id: &str, group: &Group, widget_registry: &WidgetRegistry) -> Markup {
    html! {
        section id=(format!("group-{group_id}")) class="service-group" data-service-group
            aria-labelledby=(format!("group-title-{group_id}")) {
            div class="section-heading group-heading" {
                h3 id=(format!("group-title-{group_id}")) { (group.name) }
            }
            @if !group.services.is_empty() {
                (group_grid(group_id, group, widget_registry))
            } @else {
                div class="empty-state compact-empty" { "No services in this group." }
            }
        }
    }
}

fn service_sections(config: &Dashboard, widget_registry: &WidgetRegistry) -> Markup {
    html! {
        section id="service-groups" aria-labelledby="services-heading" class="space-y-5" {
            h2 id="services-heading" class="text-lg font-semibold tracking-tight" { "Services" }
            div class="service-groups space-y-4" {
                @for (group_id, group) in &config.groups {
                    (group_section(group_id, group, widget_registry))
                }
            }
        }
    }
}

/// A small server-rendered host keeps existing configurable service widgets
/// and their HTMX refresh intervals intact while the dashboard shell is React.
pub async fn dashboard_services(Extension(state): Extension<Arc<AppState>>) -> Markup {
    let config = state.config.snapshot();
    service_sections(&config, &state.widget_registry)
}

pub async fn dashboard() -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                meta name="color-scheme" content="light dark";
                meta name="theme-color" content="#0a0a0a";
                link rel="stylesheet" type="text/css" href="/static/ui/app.css";
                script src="/static/htmx.min.js" defer {}
                script src="/static/app.js" defer {}
                script type="module" src="/static/ui/app.js" {}
                title { "Vesta" }
            }
            body {
                div id="root" {}
            }
        }
    }
}
