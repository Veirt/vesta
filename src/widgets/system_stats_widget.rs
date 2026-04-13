use async_trait::async_trait;
use maud::{Markup, html};
use std::sync::Arc;

use crate::{
    AppState,
    config::{Service, Widget},
    error::{VestaError, VestaResult},
    widget_system::{WidgetHandler, WidgetQuery},
    widgets::widget_container,
};

pub struct SystemStatsWidget;

impl SystemStatsWidget {
    pub fn new() -> Self {
        Self
    }

    fn render_progress_bar(&self, value: f64, max: f64, color_class: &str) -> Markup {
        let percentage = (value / max * 100.0).min(100.0);
        html! {
            div class="w-full bg-zinc-800 rounded-full h-1.5" {
                div class=(format!("h-1.5 rounded-full transition-all duration-300 {}", color_class))
                     style=(format!("width: {}%", percentage)) {}
            }
        }
    }

    fn get_usage_color(&self, percentage: f64) -> &'static str {
        match percentage {
            p if p < 50.0 => "bg-emerald-500",
            p if p < 80.0 => "bg-amber-400",
            _ => "bg-red-500",
        }
    }
}

#[async_trait]
impl WidgetHandler for SystemStatsWidget {
    fn name(&self) -> &'static str {
        "SystemStats"
    }

    fn render(&self, group_id: &str, service: &Service) -> Markup {
        let refresh_interval = service
            .widget
            .as_ref()
            .and_then(|w| w.config.as_ref())
            .and_then(|c| c.get("refresh_interval"))
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(5);

        widget_container(
            service.width,
            service.height,
            "overflow-y-auto",
            html! {
                div
                    class="h-full"
                    hx-get=(format!("/api/widgets/SystemStats?group={}&title={}", group_id, service.title))
                    hx-trigger=(format!("load, every {}s", refresh_interval))
                    hx-swap="innerHTML" {
                    div class="flex items-center justify-center h-full" {
                        div class="animate-spin rounded-full h-6 w-6 border-b-2 border-violet-500" {}
                    }
                }
            },
        )
    }

    async fn handle_request(
        &self,
        state: Arc<AppState>,
        _query: WidgetQuery,
    ) -> VestaResult<Markup> {
        let stats = state.get_system_stats_service().get_snapshot().await;

        Ok(html! {
            div class="space-y-4" {
                div class="flex items-center justify-between mb-4" {
                    h3 class="text-sm font-semibold text-zinc-100" style="font-family: 'JetBrains Mono', monospace;" { "System Stats" }
                    div class="text-xs text-zinc-500 font-mono" {
                        "load " (format!("{:.2}", stats.load_avg))
                    }
                }

                div class="space-y-1.5" {
                    div class="flex justify-between items-center" {
                        span class="text-xs text-zinc-400 uppercase tracking-wide" { "CPU" }
                        span class="text-xs text-zinc-200 font-mono" { (format!("{:.1}%", stats.cpu_usage)) }
                    }
                    (self.render_progress_bar(stats.cpu_usage_percent(), 100.0, self.get_usage_color(stats.cpu_usage_percent())))
                }

                div class="space-y-1.5" {
                    div class="flex justify-between items-center" {
                        span class="text-xs text-zinc-400 uppercase tracking-wide" { "Memory" }
                        span class="text-xs text-zinc-200 font-mono" {
                            (format!("{:.1}% · {}M / {}M", stats.memory_usage, stats.memory_used, stats.memory_total))
                        }
                    }
                    (self.render_progress_bar(stats.memory_usage_percent(), 100.0, self.get_usage_color(stats.memory_usage_percent())))
                }

                div class="space-y-1.5" {
                    div class="flex justify-between items-center" {
                        span class="text-xs text-zinc-400 uppercase tracking-wide" { "Disk (/)" }
                        span class="text-xs text-zinc-200 font-mono" {
                            (format!("{:.1}% · {}G / {}G", stats.disk_usage, stats.disk_used, stats.disk_total))
                        }
                    }
                    (self.render_progress_bar(stats.disk_usage_percent(), 100.0, self.get_usage_color(stats.disk_usage_percent())))
                }
            }
        })
    }

    fn validate_config(&self, widget: &Widget) -> VestaResult<()> {
        if let Some(config) = &widget.config
            && let Some(interval) = config.get("refresh_interval")
        {
            if let Ok(interval_val) = interval.parse::<u64>() {
                if !(1..=3600).contains(&interval_val) {
                    return Err(VestaError::Internal(
                        "refresh_interval must be between 1 and 3600 seconds".to_string(),
                    ));
                }
            } else {
                return Err(VestaError::Internal(
                    "refresh_interval must be a number".to_string(),
                ));
            }
        }
        Ok(())
    }
}
