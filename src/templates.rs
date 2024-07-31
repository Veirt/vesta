use crate::config::{Group, Service, Widget};
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
                script src="https://unpkg.com/htmx.org@2.0.1" integrity="sha384-QWGpdj554B4ETpJJC9z+ZHJcA/i59TyjxEPXiiUgN2WmTyV5OEZWCD6gQhgkdpB/" crossorigin="anonymous" {}
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

fn render_service_card(config: &Service) -> Markup {
    let img_src = config.img_src.as_deref().unwrap_or_default();
    let href = config.href.as_deref().unwrap_or_default();
    let width = &config.width.unwrap_or(1);
    let height = &config.height.unwrap_or(1);

    html! {
        a data-width=(width) data-height=(height) href=(href) class="flex flex-col justify-center items-center text-xs bg-black-2 rounded-xl py-2 m-2 hover:scale-105 duration-150" {
            img class="object-contain my-3 w-[2rem] h-[2rem]" src=(img_src) alt=(config.title);
            p class="text-center" { (config.title) }
        }
    }
}

fn group(group_id: &str, config: &Group) -> Markup {
    html! {
        div class="m-5" {
            p class="block ml-4 font-bold" { (config.name) }
            div.grid data-columns=(&config.columns) {
                @for service in &config.services {
                    @if let Some(widget) = &service.widget {
                        (render_widget_card(group_id, service, widget))
                    } @else {
                        (render_service_card(service))
                    }
                }
            }
        }
    }
}

pub async fn dashboard(Extension(state): Extension<Arc<AppState>>) -> Markup {
    html! {
        (head())
        body class="flex justify-center items-center min-h-screen text-white bg-black" {
            div class="container flex flex-row flex-wrap justify-center h-screen sm:justify-start"    {
                @for (id, group_config) in &state.config.groups {
                    (group(id, group_config))
                }
            }


        }
    }
}
