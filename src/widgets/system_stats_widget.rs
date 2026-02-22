use async_trait::async_trait;
use maud::{Markup, html};
use std::sync::Arc;
use sysinfo::{Disks, System};

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

    fn get_system_stats(&self) -> VestaResult<SystemStats> {
        let mut sys = System::new_all();
        sys.refresh_all();

        let cpu_usage = sys.global_cpu_usage();
        let memory_used = sys.used_memory();
        let memory_total = sys.total_memory();
        let memory_usage = (memory_used as f64 / memory_total as f64) * 100.0;

        // Get disk usage for root partition
        let mut disk_usage = 0.0;
        let mut disk_total = 0;
        let mut disk_used = 0;

        let disks = Disks::new_with_refreshed_list();
        for disk in disks.list() {
            if disk.mount_point().to_str() == Some("/") {
                disk_total = disk.total_space();
                disk_used = disk_total - disk.available_space();
                disk_usage = (disk_used as f64 / disk_total as f64) * 100.0;
                break;
            }
        }

        // Get load average (Linux only)
        let load_avg = System::load_average();

        Ok(SystemStats {
            cpu_usage,
            memory_usage,
            memory_used: memory_used / 1024 / 1024, // Convert to MB
            memory_total: memory_total / 1024 / 1024, // Convert to MB
            disk_usage,
            disk_used: disk_used / 1024 / 1024 / 1024, // Convert to GB
            disk_total: disk_total / 1024 / 1024 / 1024, // Convert to GB
            load_avg: load_avg.one,
        })
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

#[derive(Debug)]
struct SystemStats {
    cpu_usage: f32,
    memory_usage: f64,
    memory_used: u64,
    memory_total: u64,
    disk_usage: f64,
    disk_used: u64,
    disk_total: u64,
    load_avg: f64,
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
        _state: Arc<AppState>,
        _query: WidgetQuery,
    ) -> VestaResult<Markup> {
        let stats = self.get_system_stats()?;

        Ok(html! {
            div class="space-y-4" {
                // Header
                div class="flex items-center justify-between mb-4" {
                    h3 class="text-sm font-semibold text-zinc-100" style="font-family: 'JetBrains Mono', monospace;" { "System Stats" }
                    div class="text-xs text-zinc-500 font-mono" {
                        "load " (format!("{:.2}", stats.load_avg))
                    }
                }

                // CPU Usage
                div class="space-y-1.5" {
                    div class="flex justify-between items-center" {
                        span class="text-xs text-zinc-400 uppercase tracking-wide" { "CPU" }
                        span class="text-xs text-zinc-200 font-mono" { (format!("{:.1}%", stats.cpu_usage)) }
                    }
                    (self.render_progress_bar(stats.cpu_usage as f64, 100.0, self.get_usage_color(stats.cpu_usage as f64)))
                }

                // Memory Usage
                div class="space-y-1.5" {
                    div class="flex justify-between items-center" {
                        span class="text-xs text-zinc-400 uppercase tracking-wide" { "Memory" }
                        span class="text-xs text-zinc-200 font-mono" {
                            (format!("{:.1}% · {}M / {}M", stats.memory_usage, stats.memory_used, stats.memory_total))
                        }
                    }
                    (self.render_progress_bar(stats.memory_usage, 100.0, self.get_usage_color(stats.memory_usage)))
                }

                // Disk Usage
                div class="space-y-1.5" {
                    div class="flex justify-between items-center" {
                        span class="text-xs text-zinc-400 uppercase tracking-wide" { "Disk (/)" }
                        span class="text-xs text-zinc-200 font-mono" {
                            (format!("{:.1}% · {}G / {}G", stats.disk_usage, stats.disk_used, stats.disk_total))
                        }
                    }
                    (self.render_progress_bar(stats.disk_usage, 100.0, self.get_usage_color(stats.disk_usage)))
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
