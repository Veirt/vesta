use crate::config::{Group, Service, Widget};
use crate::ping::render_service_indicator;
use crate::{widgets, AppState};
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

            }
        }
    }
}

fn render_widget_card(group_id: &str, service: &Service, widget: &Widget) -> Markup {
    match widget.name.as_str() {
        "SonarrCalendar" => {
            widgets::sonarr_calendar::render_sonarr_calendar_widget(group_id, service)
        }
        _ => html!(),
    }
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

fn group(group_id: &str, config: &Group) -> Markup {
    html! {
        div class="container"  {
            h2 class="text-sky-400 block font-bold text-lg my-2" { (config.name) }
            div class=(format!("grid grid-cols-{} gap-4", &config.columns)) {
                @for service in &config.services {
                    @if let Some(widget) = &service.widget {
                        (render_widget_card(group_id, service, widget))
                    } @else {
                        (render_service_card(group_id, service))
                    }
                }
            }
        }
    }
}

pub async fn dashboard(Extension(state): Extension<Arc<AppState>>) -> Markup {
    // Reload config
    if let Err(e) = state.reload_config() {
        eprintln!("Error reloading config: {}", e);
    }

    // Get the latest config
    let config = state.get_config();
    html! {
        (head())
        body class="min-h-screen text-white bg-slate-950 flex" {

            aside."w-64 bg-slate-900 p-4 fixed h-full" {
                h1."text-2xl font-bold mx-4 my-4" { "Vesta" }
            }

            div."flex-1 ml-72 mr-8 mt-4" {
                header."container" {
                    section {
                        h2."text-2xl font-bold" { "Services" }
                        p."text-stone-400" { "Your self-hosted applications" }
                    }
                }

                main class="container my-4 !mb-[20px] gap-2 flex flex-wrap justify-center h-screen lg:justify-start" {
                    @for (id, group_config) in &config.groups {
                        (group(id, group_config))
                    }
                }
            }
        }
    }
}
