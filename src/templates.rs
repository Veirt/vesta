use crate::config::{Group, Service, Widget};
use crate::ping::render_service_indicator;
use crate::AppState;
use axum::Extension;
use maud::{html, Markup, DOCTYPE};
use std::sync::Arc;

fn head() -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                link rel="stylesheet" type="text/css" href="/static/style.css";
                script src="static/htmx.min.js" integrity="sha384-HGfztofotfshcF7+8n44JQL2oJmowVChPTg48S+jvZoztPfvwD79OC/LTtG6dMp+" crossorigin="anonymous" {}
                title { "Vesta" }
                style {
                    "html { scroll-behavior: smooth; }"
                }
            }
        }
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

    html! {
        a href=(href) target="_blank" rel="noreferrer" class=(format!("col-span-{} row-span-{} flex flex-row p-4 justify-between items-center text-xs bg-slate-900 border border-slate-800 rounded-xl hover:scale-105 duration-150", width, height)) {
            img class="object-contain my-3 w-[2rem] h-[2rem]" src=(img_src) alt=(service_info.title);

            p class="text-center" { (service_info.title) }

            @if service_info.ping.is_some() {
                (render_service_indicator(group_id, &service_info.title))
            }
        }
    }
}

fn group(
    group_id: &str,
    config: &Group,
    widget_registry: &crate::widget_system::WidgetRegistry,
) -> Markup {
    html! {
        div id=(group_id) class="container scroll-mt-6"  {
            h2 class="text-sky-400 block font-bold text-lg my-2" { (config.name) }
            div class=(format!("grid grid-cols-{} gap-4", &config.columns)) {
                @for service in &config.services {
                    @if let Some(widget) = &service.widget {
                        (render_widget_card(group_id, service, widget, widget_registry))
                    } @else {
                        (render_service_card(group_id, service))
                    }
                }
            }
        }
    }
}

pub async fn dashboard(Extension(state): Extension<Arc<AppState>>) -> Markup {
    if let Err(e) = state.reload_config() {
        eprintln!("Error reloading config: {}", e);
    }

    let config_result = state.get_config_manager().read_config();
    let config = match config_result {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Error getting config: {}", e);
            return html! {
                (head())
                body class="min-h-full text-white bg-slate-950 flex items-center justify-center" {
                    div class="text-center" {
                        h1 class="text-2xl font-bold mb-4" { "Configuration Error" }
                        p class="text-red-400" { (e.to_string()) }
                    }
                }
            };
        }
    };
    html! {
        (head())
        body class="min-h-full text-white bg-slate-950 flex" {

            aside."w-64 bg-slate-900 p-4 fixed h-full overflow-y-auto" {
                div class="flex flex-col h-full" {
                    div class="flex items-center gap-3 mx-4 my-4 pb-4 border-b border-slate-700" {
                        img src="/static/logo-white.png" alt="Vesta" class="w-8 h-8";
                        h1."text-2xl font-bold" { "Vesta" }
                    }

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

                    div class="mt-auto pt-4 border-t border-slate-700" {
                        h3 class="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3 px-4" { "Status" }
                        div class="px-4 space-y-2" {
                            @let total_services = config.groups.values().map(|g| g.services.len()).sum::<usize>();
                            @let services_with_ping = config.groups.values().flat_map(|g| &g.services).filter(|s| s.ping.is_some()).count();

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

            div."flex-1 ml-72 mr-8 my-4 min-h-full" {
                header."container" {
                    section {
                        h2."text-2xl font-bold" { "Services" }
                        p."text-stone-400" { "Your self-hosted applications" }
                    }
                }

                main class="container my-4 gap-2 flex flex-wrap justify-center h-full lg:justify-start" {
                    @for (id, group_config) in &config.groups {
                        (group(id, group_config, state.get_widget_registry()))
                    }
                }
            }
        }
    }
}
