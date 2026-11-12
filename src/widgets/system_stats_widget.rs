use async_trait::async_trait;
use maud::{Markup, html};
use serde::Deserialize;

use crate::config::Service;
use crate::error::VestaResult;
use crate::widget_system::{
    WidgetContext, WidgetHandler, WidgetRef, stat_row, usage_bar, widget_scaffold_with,
};

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    /// Node id from `[nodes]` to display; empty picks the first configured node.
    #[serde(default)]
    pub node: String,
    /// Refresh cadence in seconds (1–3600).
    #[serde(
        default = "default_refresh",
        deserialize_with = "crate::widget_config::debounced_u64"
    )]
    pub refresh_interval: u64,
}

fn default_refresh() -> u64 {
    5
}

pub struct SystemStatsWidget;

impl SystemStatsWidget {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SystemStatsWidget {
    fn default() -> Self {
        Self::new()
    }
}

fn fmt_mib(bytes: u64) -> String {
    format!("{}M", bytes / 1024 / 1024)
}

fn fmt_gib(bytes: u64) -> String {
    format!("{}G", bytes / 1024 / 1024 / 1024)
}

fn stats_body(name: &str, stats: &crate::metrics::node::NodeStats) -> Markup {
    let load = stats
        .load
        .map(|l| format!("{:.2}", l.one))
        .unwrap_or_else(|| "n/a".to_string());
    html! {
        div class="space-y-4" {
            div class="flex items-center justify-between mb-4" {
                h3 class="text-sm font-semibold text-zinc-100" style="font-family: 'JetBrains Mono', monospace;" {
                    (name)
                }
                div class="text-xs text-zinc-500 font-mono" {
                    "load " (load)
                }
            }
            (stat_row(
                "CPU",
                &format!("{:.1}%", stats.cpu_usage_percent),
                Some(usage_bar(stats.cpu_usage_percent)),
            ))
            (stat_row(
                "Memory",
                &format!("{:.1}% · {} / {}",
                    stats.memory_usage_percent(),
                    fmt_mib(stats.memory_used_bytes),
                    fmt_mib(stats.memory_total_bytes)),
                Some(usage_bar(stats.memory_usage_percent())),
            ))
            (stat_row(
                &format!("Disk ({})", stats.disk_mount),
                &format!("{:.1}% · {} / {}",
                    stats.disk_usage_percent(),
                    fmt_gib(stats.disk_used_bytes),
                    fmt_gib(stats.disk_total_bytes)),
                Some(usage_bar(stats.disk_usage_percent())),
            ))
            div class="flex justify-between text-xs pt-2 border-t border-zinc-800" {
                span class="text-zinc-500" { "Uptime" }
                span class="text-zinc-300 font-mono" { (format_uptime(stats.uptime_secs)) }
            }
        }
    }
}

fn format_uptime(secs: u64) -> String {
    let days = secs / 86_400;
    let hours = (secs % 86_400) / 3_600;
    let mins = (secs % 3_600) / 60;
    if days > 0 {
        format!("{days}d {hours}h {mins}m")
    } else {
        format!("{hours}h {mins}m")
    }
}

#[async_trait]
impl WidgetHandler for SystemStatsWidget {
    fn name(&self) -> &'static str {
        "SystemStats"
    }

    type Config = Config;

    fn render(&self, mount: &WidgetRef, service: &Service, config: &Config) -> Markup {
        let frame = html! {
            div class="flex items-center justify-center h-full" {
                p class="text-xs text-zinc-500" { "Waiting for first poll…" }
            }
        };
        widget_scaffold_with(
            service,
            mount,
            self.name(),
            Some(config.refresh_interval),
            frame,
        )
    }

    async fn handle_request(
        &self,
        ctx: WidgetContext,
        _mount: WidgetRef,
        config: Config,
    ) -> VestaResult<Markup> {
        let node_id = if config.node.is_empty() {
            ctx.nodes.node_ids().await.first().cloned().ok_or_else(|| {
                crate::error::VestaError::InvalidWidgetConfig(
                    "no nodes configured; add a [nodes] entry to vesta.toml".to_string(),
                )
            })?
        } else {
            config.node.clone()
        };

        let stats = ctx.nodes.stats_for(&node_id).await?;
        let node_name = ctx
            .config
            .snapshot()
            .nodes
            .iter()
            .find(|n| n.id == node_id)
            .map(|n| n.name.clone())
            .unwrap_or_else(|| node_id.clone());

        Ok(stats_body(&node_name, &stats))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_defaults_refresh_to_5s() {
        let config: Config = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(config.refresh_interval, 5);
        assert_eq!(config.node, "");
    }

    #[test]
    fn config_accepts_string_numbers_like_toml() {
        let config: Config =
            serde_json::from_value(serde_json::json!({"refresh_interval": "10", "node": "nas"}))
                .unwrap();
        assert_eq!(config.refresh_interval, 10);
        assert_eq!(config.node, "nas");
    }

    #[test]
    fn formats_uptime() {
        assert_eq!(format_uptime(0), "0h 0m");
        assert_eq!(format_uptime(3_600), "1h 0m");
        assert_eq!(format_uptime(93_660), "1d 2h 1m");
    }
}
