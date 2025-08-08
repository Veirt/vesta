use async_trait::async_trait;
use chrono::{Local, Utc};
use maud::{html, Markup};
use std::sync::Arc;

use crate::{
    config::{Service, Widget},
    error::VestaResult,
    widget_system::{WidgetHandler, WidgetQuery},
    widgets::widget_container,
    AppState,
};

pub struct ClockWidget;

impl ClockWidget {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl WidgetHandler for ClockWidget {
    fn name(&self) -> &'static str {
        "Clock"
    }

    fn render(&self, _group_id: &str, service: &Service) -> Markup {
        let now = Local::now();
        let utc_now = Utc::now();

        widget_container(
            service.width,
            service.height,
            "flex flex-col justify-center",
            html! {
                div data-clock class="text-center space-y-2" {
                    // Main time display
                    div data-clock-time class="text-4xl font-mono font-bold text-white" {
                        (now.format("%H:%M:%S").to_string())
                    }

                    // Date display
                    div data-clock-date class="text-lg text-gray-300" {
                        (now.format("%A, %B %d").to_string())
                    }

                    // Year and timezone
                    div class="text-sm text-gray-400 space-y-1" {
                        div data-clock-year { (now.format("%Y").to_string()) }
                        div data-clock-tz { (now.format("%Z").to_string()) }
                    }

                    // UTC time
                    div data-clock-utc class="text-xs text-gray-500 pt-2 border-t border-slate-800" {
                        "UTC: " (utc_now.format("%H:%M:%S").to_string())
                    }
                }
            },
        )
    }

    async fn handle_request(
        &self,
        _state: Arc<AppState>,
        _query: WidgetQuery,
    ) -> VestaResult<Markup> {
        let now = Local::now();
        let utc_now = Utc::now();

        Ok(html! {
            div class="text-center space-y-2" {
                // Main time display
                div class="text-4xl font-mono font-bold text-white" {
                    (now.format("%H:%M:%S").to_string())
                }

                // Date display
                div class="text-lg text-gray-300" {
                    (now.format("%A, %B %d").to_string())
                }

                // Year and timezone
                div class="text-sm text-gray-400 space-y-1" {
                    div { (now.format("%Y").to_string()) }
                    div { (now.format("%Z").to_string()) }
                }

                // UTC time
                div class="text-xs text-gray-500 pt-2 border-t border-slate-800" {
                    "UTC: " (utc_now.format("%H:%M:%S").to_string())
                }
            }
        })
    }

    fn validate_config(&self, _widget: &Widget) -> VestaResult<()> {
        // Clock widget doesn't need any configuration
        Ok(())
    }
}
