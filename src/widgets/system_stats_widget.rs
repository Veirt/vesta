use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::error::VestaResult;
use crate::widget_system::{WidgetContext, WidgetField, WidgetHandler, WidgetQuery};

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

#[async_trait]
impl WidgetHandler for SystemStatsWidget {
    fn name(&self) -> &str {
        "SystemStats"
    }

    fn description(&self) -> &str {
        "Shows CPU, memory, disk, load, and uptime for one configured server."
    }

    fn catalog_visible(&self) -> bool {
        false
    }

    fn default_size(&self) -> (u8, u8) {
        (6, 4)
    }

    fn min_size(&self) -> (u8, u8) {
        (4, 4)
    }

    fn fields(&self) -> Vec<WidgetField> {
        vec![
            WidgetField::builtin(
                "node",
                "Server",
                "node",
                false,
                "Leave on the first server to follow the configured order.",
            )
            .with_default(serde_json::json!("")),
            WidgetField::builtin(
                "refresh_interval",
                "Refresh every (seconds)",
                "number",
                true,
                "How often this widget requests updated values (1–3600 seconds).",
            )
            .with_default(serde_json::json!(5))
            .with_range(1.0, 3600.0),
        ]
    }

    type Config = Config;

    fn refresh_interval_secs(&self, config: &Config) -> Option<u64> {
        Some(config.refresh_interval)
    }

    async fn handle_request(
        &self,
        ctx: WidgetContext,
        _query: WidgetQuery,
        config: Config,
    ) -> VestaResult<Value> {
        let node_id = if config.node.is_empty() {
            ctx.nodes.node_ids().first().cloned().ok_or_else(|| {
                crate::error::VestaError::InvalidWidgetConfig(
                    "no nodes configured; add a server in Settings".to_string(),
                )
            })?
        } else {
            config.node.clone()
        };

        let stats = ctx.nodes.stats_for(&node_id).await?;
        let node_name = ctx
            .nodes
            .node_name(&node_id)
            .unwrap_or_else(|| node_id.clone());

        Ok(json!({
            "name": node_name,
            "stats": {
                "cpu_usage_percent": stats.cpu_usage_percent,
                "memory": {
                    "used_bytes": stats.memory_used_bytes,
                    "total_bytes": stats.memory_total_bytes,
                    "usage_percent": stats.memory_usage_percent(),
                },
                "disk": {
                    "mount": stats.disk_mount,
                    "used_bytes": stats.disk_used_bytes,
                    "total_bytes": stats.disk_total_bytes,
                    "usage_percent": stats.disk_usage_percent(),
                },
                "load": stats.load.map(|load| json!({
                    "one": load.one,
                    "five": load.five,
                    "fifteen": load.fifteen,
                })),
                "uptime_secs": stats.uptime_secs,
            },
        }))
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
    fn config_accepts_numeric_strings() {
        let config: Config =
            serde_json::from_value(serde_json::json!({"refresh_interval": "10", "node": "nas"}))
                .unwrap();
        assert_eq!(config.refresh_interval, 10);
        assert_eq!(config.node, "nas");
    }
}
