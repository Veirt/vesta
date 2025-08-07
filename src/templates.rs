use crate::config::{Dashboard, Group, Service, Widget};
use crate::ping::render_service_indicator;
use crate::AppState;
use axum::Extension;
use maud::{html, Markup, DOCTYPE};
use std::sync::Arc;

// HTML head component with mobile-first responsive design
fn head() -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                link rel="stylesheet" type="text/css" href="/static/style.css";
                script src="static/htmx.min.js" integrity="sha384-HGfztofotfshcF7+8n44JQL2oJmowVChPTg48S+jvZoztPfvwD79OC/LTtG6dMp+" crossorigin="anonymous" {}
                script src="static/app.js" {}
                title { "Vesta" }
                style {
                    "html { scroll-behavior: smooth; }"
                    ".mobile-menu-overlay { position: fixed; top: 0; left: 0; width: 100%; height: 100%; background: rgba(0,0,0,0.5); z-index: 30; display: none; }"
                    ".mobile-menu-open .mobile-menu-overlay { display: block; }"
                    ".mobile-menu-open #sidebar { transform: translateX(0) !important; }"
                }
            }
        }
    }
}

// Mobile menu button component
fn mobile_nav_toggle() -> Markup {
    html! {
        div class="md:hidden fixed top-4 left-4 z-50" {
            button
                id="mobile-menu-toggle"
                class="bg-slate-800 text-white p-2 rounded-lg shadow-lg" {
                svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24" {
                    path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16" {}
                }
            }
        }
    }
}

fn sidebar_logo() -> Markup {
    html! {
        div class="flex items-center gap-3 mx-4 my-4 pb-4 border-b border-slate-700" {
            img src="/static/logo-white.png" alt="Vesta" class="w-8 h-8";
            h1."text-2xl font-bold" { "Vesta" }

        }
    }
}

fn sidebar_navigation(config: &Dashboard) -> Markup {
    html! {
        nav class="flex-1 mt-6" {
            h3 class="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3 px-4" { "Groups" }
            div class="space-y-1" {
                @for (group_id, group_config) in &config.groups {
                    a href=(format!("#{}", group_id))
                      class="flex items-center px-4 py-2 text-sm text-slate-300 hover:bg-slate-800 hover:text-white rounded-lg transition-colors group" {
                        div class="w-2 h-2 bg-sky-400 rounded-full mr-3 group-hover:bg-sky-300" {}
                        span { (group_config.name) }
                        span class="ml-auto text-xs text-slate-500" { (group_config.services.len()) }
                    }
                }
            }
        }
    }
}

fn sidebar_status(config: &Dashboard) -> Markup {
    let total_services = config
        .groups
        .values()
        .map(|g| g.services.len())
        .sum::<usize>();
    let services_with_ping = config
        .groups
        .values()
        .flat_map(|g| &g.services)
        .filter(|s| s.ping.is_some())
        .count();

    html! {
        div class="mt-auto pt-4 border-t border-slate-700" {
            h3 class="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3 px-4" { "Status" }
            div class="px-4 space-y-2" {
                div class="flex justify-between text-sm" {
                    span class="text-slate-400" { "Total Services" }
                    span class="text-white font-medium" { (total_services) }
                }
                div class="flex justify-between text-sm" {
                    span class="text-slate-400" { "Monitored" }
                    span class="text-white font-medium" { (services_with_ping) }
                }
                div class="flex justify-between text-sm" {
                    span class="text-slate-400" { "Groups" }
                    span class="text-white font-medium" { (config.groups.len()) }
                }
            }
        }
    }
}

fn sidebar(config: &Dashboard) -> Markup {
    html! {
        aside
            id="sidebar"
            class="w-64 bg-slate-900 p-4 fixed h-full overflow-y-auto z-40 -translate-x-full md:translate-x-0 transition-transform duration-300 ease-in-out" {
            div class="flex flex-col h-full" {
                // Close button for mobile
                div class="md:hidden flex justify-end mb-4" {
                    button
                        id="mobile-menu-close"
                        class="p-2 text-slate-400 hover:text-white" {
                        svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24" {
                            path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" {}
                        }
                    }
                }
                (sidebar_logo())
                (sidebar_navigation(config))
                (sidebar_status(config))
            }
        }
    }
}

fn main_header() -> Markup {
    html! {
        header class="mb-8" {
            div class="flex items-center justify-between" {
                div {
                    h1 class="text-4xl font-bold bg-gradient-to-r from-white via-slate-200 to-slate-400 bg-clip-text text-transparent mb-2" {
                        "Services"
                    }
                    p class="text-slate-400 text-lg font-medium flex items-center gap-2" {
                        svg class="w-5 h-5 text-sky-400" fill="none" stroke="currentColor" viewBox="0 0 24 24" {
                            path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" {}
                        }
                        "Your self-hosted applications"
                    }
                }
            }
        }
    }
}

fn main_content(
    config: &Dashboard,
    widget_registry: &crate::widget_system::WidgetRegistry,
) -> Markup {
    html! {
        div class="flex-1 ml-0 md:ml-64 px-4 md:px-8 py-4 min-h-full" {
            (main_header())
            main class="container mx-auto my-4 gap-2 flex flex-wrap justify-center h-full lg:justify-start" {
                @for (id, group_config) in &config.groups {
                    (group(id, group_config, widget_registry))
                }
            }
        }
    }
}

fn error_page(error_message: &str) -> Markup {
    html! {
        (head())
        body class="min-h-full text-white bg-slate-950 flex items-center justify-center" {
            div class="text-center" {
                h1 class="text-2xl font-bold mb-4" { "Configuration Error" }
                p class="text-red-400" { (error_message) }
            }
        }
    }
}

fn service_card_image(img_src: &str, title: &str) -> Markup {
    html! {
        img class="object-contain w-6 h-6 md:w-8 md:h-8 mb-1 md:mb-0 md:my-3 !mt-0"
            src=(img_src)
            alt=(title);
    }
}

fn service_card_title(title: &str) -> Markup {
    html! {
        p class="text-center text-xs font-medium" { (title) }
    }
}

fn render_widget_card(
    group_id: &str,
    service: &Service,
    widget: &Widget,
    widget_registry: &crate::widget_system::WidgetRegistry,
) -> Markup {
    widget_registry.render_widget(group_id, service, widget)
}

fn render_service_card(group_id: &str, service_info: &Service) -> Markup {
    let img_src = service_info.img_src.as_deref().unwrap_or_default();
    let href = service_info.href.as_deref().unwrap_or_default();
    let width = service_info.width.unwrap_or(1);
    let height = service_info.height.unwrap_or(1);
    let has_ping = service_info.ping.is_some();

    html! {
        a href=(href)
          target="_blank"
          rel="noreferrer"
          class=(format!("relative col-span-1 md:col-span-{} row-span-1 md:row-span-{} flex flex-col  xl:flex-row  p-3 md:p-4 justify-center md:justify-between items-center text-xs bg-slate-900 border border-slate-800 rounded-xl hover:scale-105 duration-150 min-h-[80px] md:min-h-0", width, height)) {
            (service_card_image(img_src, &service_info.title))
            (service_card_title(&service_info.title))
            @if has_ping {
                div class="absolute top-2 right-2 xl:static xl:top-auto xl:right-auto" {
                    (render_service_indicator(group_id, &service_info.title))
                }
            }
        }
    }
}

fn group_header(group_name: &str) -> Markup {
    html! {
        div class="mb-6 flex items-center gap-3" {
            div class="w-1 h-8 bg-gradient-to-b from-sky-400 to-blue-500 rounded-full" {}
            h2 class="text-xl font-bold text-white" { (group_name) }
            div class="flex-1 h-px bg-gradient-to-r from-slate-700 to-transparent" {}
        }
    }
}

fn group_grid(
    group_id: &str,
    group_config: &Group,
    widget_registry: &crate::widget_system::WidgetRegistry,
) -> Markup {
    html! {
        div class=(format!("grid grid-cols-2 sm:grid-cols-{} gap-4", &group_config.columns)) {
            @for service in &group_config.services {
                (render_service_or_widget(group_id, service, widget_registry))
            }
        }
    }
}

fn render_service_or_widget(
    group_id: &str,
    service: &Service,
    widget_registry: &crate::widget_system::WidgetRegistry,
) -> Markup {
    if let Some(widget) = &service.widget {
        render_widget_card(group_id, service, widget, widget_registry)
    } else {
        render_service_card(group_id, service)
    }
}

fn group(
    group_id: &str,
    config: &Group,
    widget_registry: &crate::widget_system::WidgetRegistry,
) -> Markup {
    html! {
        div id=(group_id) class="container scroll-mt-6" {
            (group_header(&config.name))
            (group_grid(group_id, config, widget_registry))
        }
    }
}

fn load_dashboard_config(
    state: &AppState,
) -> Result<std::sync::RwLockReadGuard<Dashboard>, String> {
    if let Err(e) = state.reload_config() {
        eprintln!("Error reloading config: {}", e);
    }

    state.get_config_manager().read_config().map_err(|e| {
        eprintln!("Error getting config: {}", e);
        e.to_string()
    })
}

pub async fn dashboard(Extension(state): Extension<Arc<AppState>>) -> Markup {
    let config = match load_dashboard_config(&state) {
        Ok(config) => config,
        Err(error_message) => return error_page(&error_message),
    };

    html! {
        (head())
        body class="min-h-full text-white bg-slate-950 flex" {
            div class="mobile-menu-overlay" {}
            (mobile_nav_toggle())
            (sidebar(&config))
            (main_content(&config, state.get_widget_registry()))
        }
    }
}
