use std::sync::Arc;

use axum::Extension;
use maud::{DOCTYPE, Markup, html};

use crate::config::{Dashboard, Group, Service};
use crate::ping::service_indicator;
use crate::widget_system::WidgetRef;
use crate::{AppState, widget_system::WidgetRegistry};

fn head() -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                link rel="stylesheet" type="text/css" href="/static/style.css";
                link rel="preconnect" href="https://fonts.googleapis.com";
                link rel="preconnect" href="https://fonts.gstatic.com" crossorigin;
                link href="https://fonts.googleapis.com/css2?family=IBM+Plex+Sans:wght@300;400;500;600;700&family=JetBrains+Mono:wght@400;500;600;700&display=swap" rel="stylesheet";
                script src="static/htmx.min.js" {}
                script src="static/app.js" {}
                title { "Vesta" }
                style {
                    "html { scroll-behavior: smooth; }"
                    "body { font-family: 'IBM Plex Sans', sans-serif; }"
                    "h1, h2, h3, .font-mono { font-family: 'JetBrains Mono', monospace; }"
                    ".mobile-menu-overlay { position: fixed; top: 0; left: 0; width: 100%; height: 100%; background: rgba(0,0,0,0.6); z-index: 30; display: none; backdrop-filter: blur(2px); }"
                    ".mobile-menu-open .mobile-menu-overlay { display: block; }"
                    ".mobile-menu-open #sidebar { transform: translateX(0) !important; }"
                }
            }
        }
    }
}

fn mobile_nav_toggle() -> Markup {
    html! {
        div class="md:hidden fixed top-4 left-4 z-50" {
            button
                id="mobile-menu-toggle"
                class="bg-zinc-900 border border-zinc-800 text-zinc-300 hover:text-white p-2 rounded-lg transition-colors duration-150 cursor-pointer" {
                svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" {
                    path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16" {}
                }
            }
        }
    }
}

fn sidebar_wordmark() -> Markup {
    html! {
        div class="flex items-center gap-2 mx-4 my-5 pb-5 border-b border-zinc-800" {
            span class="text-lg font-bold tracking-tight text-white"
                style="font-family: 'JetBrains Mono', monospace;" {
                "vesta"
            }
            span class="text-xs text-zinc-500 font-normal mt-0.5"
                style="font-family: 'IBM Plex Sans', sans-serif;" {
                "dashboard"
            }
        }
    }
}

fn sidebar_navigation(config: &Dashboard) -> Markup {
    html! {
        nav class="flex-1 mt-4" {
            p class="text-xs font-medium text-zinc-500 uppercase tracking-widest mb-3 px-4" { "Groups" }
            div class="space-y-0.5" {
                @for (group_id, group_config) in &config.groups {
                    a href=(format!("#{group_id}"))
                      class="flex items-center px-4 py-2 text-sm text-zinc-400 hover:bg-zinc-800/60 hover:text-zinc-100 rounded-md transition-colors duration-150 cursor-pointer group" {
                        span class="w-1.5 h-1.5 rounded-full bg-violet-500/50 group-hover:bg-violet-400 mr-3 flex-shrink-0 transition-colors duration-150" {}
                        span class="flex-1 truncate" { (group_config.name) }
                        span class="text-xs text-zinc-600 group-hover:text-zinc-400 transition-colors duration-150 font-mono" {
                            (group_config.services.len())
                        }
                    }
                }
            }
        }
    }
}

fn sidebar_status(config: &Dashboard) -> Markup {
    let total_services = config.services().count();
    let services_with_ping = config
        .services()
        .filter(|(_, _, s)| s.ping.is_some())
        .count();

    html! {
        div class="mt-auto pt-4 border-t border-zinc-800" {
            p class="text-xs font-medium text-zinc-500 uppercase tracking-widest mb-3 px-4" { "Status" }
            div class="px-4 space-y-2" {
                div class="flex justify-between text-xs" {
                    span class="text-zinc-500" { "Services" }
                    span class="text-zinc-300 font-mono" { (total_services) }
                }
                div class="flex justify-between text-xs" {
                    span class="text-zinc-500" { "Monitored" }
                    span class="text-zinc-300 font-mono" { (services_with_ping) }
                }
                div class="flex justify-between text-xs" {
                    span class="text-zinc-500" { "Nodes" }
                    span class="text-zinc-300 font-mono" { (config.nodes.len()) }
                }
            }
        }
    }
}

fn sidebar(config: &Dashboard) -> Markup {
    html! {
        aside
            id="sidebar"
            class="w-56 bg-zinc-950 border-r border-zinc-800/60 p-3 fixed h-full overflow-y-auto z-40 -translate-x-full md:translate-x-0 transition-transform duration-300 ease-in-out" {
            div class="flex flex-col h-full" {
                div class="md:hidden flex justify-end mb-2" {
                    button
                        id="mobile-menu-close"
                        class="p-2 text-zinc-500 hover:text-zinc-200 transition-colors duration-150 cursor-pointer" {
                        svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" {
                            path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" {}
                        }
                    }
                }
                (sidebar_wordmark())
                (sidebar_navigation(config))
                (sidebar_status(config))
            }
        }
    }
}

fn main_header() -> Markup {
    html! {
        header class="mb-8" {
            h1 class="text-2xl font-semibold text-zinc-100 tracking-tight mb-1"
                style="font-family: 'JetBrains Mono', monospace;" {
                "Services"
            }
            p class="text-sm text-zinc-500 flex items-center gap-1.5" {
                svg class="w-4 h-4 text-zinc-600" fill="none" stroke="currentColor" viewBox="0 0 24 24" {
                    path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 12h14M12 5l7 7-7 7" {}
                }
                "Self-hosted applications"
            }
        }
    }
}

fn service_card(group_id: &str, service: &Service) -> Markup {
    let img_src = service.img_src.as_deref().unwrap_or_default();
    let href = service.href.as_deref().unwrap_or_default();
    let width = service.width.unwrap_or(1).max(1);
    let height = service.height.unwrap_or(1).max(1);

    html! {
        a href=(href)
          target="_blank"
          rel="noreferrer"
          class="relative h-full flex flex-col xl:flex-row p-3 md:p-4 justify-center md:justify-between items-center text-xs bg-zinc-900 border border-zinc-800 rounded-lg hover:border-violet-500/40 hover:bg-zinc-800/80 transition-all duration-150 cursor-pointer"
          style=(format!("grid-column: span {width}; grid-row: span {height};")) {
            img class="object-contain w-6 h-6 md:w-7 md:h-7 mb-1 md:mb-0 md:my-2 !mt-0 opacity-90"
                src=(img_src)
                alt=(service.title);
            p class="text-center text-xs text-zinc-300 font-medium leading-tight" { (service.title) }
            @if service.ping.is_some() {
                div class="absolute top-2 right-2 xl:static xl:top-auto xl:right-auto" {
                    (service_indicator(group_id, &service.title))
                }
            }
        }
    }
}

fn group_grid(group_id: &str, group: &Group, widget_registry: &WidgetRegistry) -> Markup {
    let has_widget = group.services.iter().any(|s| s.widget.is_some());
    let base_cols = if has_widget { 1 } else { 2 };
    html! {
        div class=(format!("grid auto-rows-[5rem] grid-cols-{base_cols} sm:grid-cols-{} gap-2 md:gap-3 items-stretch", group.columns)) {
            @for service in &group.services {
                @if let Some(widget) = &service.widget {
                    (widget_registry.render_widget(
                        &WidgetRef::new(group_id, &service.title),
                        service,
                        widget,
                    ))
                } @else {
                    (service_card(group_id, service))
                }
            }
        }
    }
}

fn group(group_id: &str, group: &Group, widget_registry: &WidgetRegistry) -> Markup {
    html! {
        div id=(group_id) class="container scroll-mt-6" {
            div class="mb-5 flex items-center gap-3" {
                span class="text-xs font-medium text-violet-400 uppercase tracking-widest"
                    style="font-family: 'JetBrains Mono', monospace;" {
                    (group.name)
                }
                div class="flex-1 h-px bg-zinc-800" {}
            }
            (group_grid(group_id, group, widget_registry))
        }
    }
}

fn main_content(config: &Dashboard, widget_registry: &WidgetRegistry) -> Markup {
    html! {
        div class="flex-1 ml-0 md:ml-56 px-6 md:px-10 py-8 min-h-full" {
            (main_header())
            main class="container mx-auto my-4 gap-4 flex flex-wrap justify-center h-full lg:justify-start" {
                @for (id, group_config) in &config.groups {
                    (group(id, group_config, widget_registry))
                }
            }
        }
    }
}

pub async fn dashboard(Extension(state): Extension<Arc<AppState>>) -> Markup {
    let config = state.config.snapshot();

    html! {
        (head())
        body class="min-h-full text-white bg-zinc-950 flex" {
            div class="mobile-menu-overlay" {}
            (mobile_nav_toggle())
            (sidebar(&config))
            (main_content(&config, &state.widget_registry))
        }
    }
}
