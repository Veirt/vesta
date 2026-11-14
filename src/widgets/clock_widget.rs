use async_trait::async_trait;
use chrono::{Local, Utc};
use maud::{Markup, html};
use serde::Deserialize;

use crate::config::Service;
use crate::error::VestaResult;

use crate::widget_system::{WidgetContext, WidgetHandler, WidgetRef, widget_scaffold_with};

/// Client-side ticking clock; the server renders the initial frame into the
/// scaffold (no loading spinner flash) and `app.js` keeps it ticking.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Config {}

pub struct ClockWidget;

impl ClockWidget {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ClockWidget {
    fn default() -> Self {
        Self::new()
    }
}

fn clock_markup(now: chrono::DateTime<Local>, utc_now: chrono::DateTime<Utc>) -> Markup {
    html! {
        div data-clock class="clock-widget" {
            div data-clock-time class="clock-time widget-value" {
                (now.format("%H:%M:%S").to_string())
            }
            div data-clock-date class="clock-date widget-secondary" {
                (now.format("%A, %B %d").to_string())
            }
            div class="clock-meta widget-secondary" {
                div data-clock-year { (now.format("%Y").to_string()) }
                div data-clock-tz { (now.format("%Z").to_string()) }
            }
            div data-clock-utc class="clock-utc widget-secondary widget-divider" {
                "UTC " (utc_now.format("%H:%M:%S").to_string())
            }
        }
    }
}

#[async_trait]
impl WidgetHandler for ClockWidget {
    fn name(&self) -> &'static str {
        "Clock"
    }

    fn description(&self) -> &'static str {
        "Shows your local time, date, time zone, and UTC time."
    }

    type Config = Config;

    fn render(&self, mount: &WidgetRef, service: &Service, _config: &Config) -> Markup {
        // The clock renders its frame directly into the scaffold; htmx swaps
        // in a fresh frame when the client triggers a reload.
        let frame = clock_markup(Local::now(), Utc::now());
        widget_scaffold_with(service, mount, self.name(), None, frame)
    }

    async fn handle_request(
        &self,
        _ctx: WidgetContext,
        _mount: WidgetRef,
        _config: Config,
    ) -> VestaResult<Markup> {
        Ok(clock_markup(Local::now(), Utc::now()))
    }
}
