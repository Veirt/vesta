use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// A monitoring target polled on a fixed interval. Each instance is a
/// standalone stats source: node_exporter URLs expose the Prometheus text
/// format; the special value `local` reads the host through sysinfo.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NodeConfig {
    /// Stable identifier used in the API (`/api/nodes/{id}/stats`).
    pub id: String,
    /// Display name shown in the widget.
    pub name: String,
    /// Optional host name shown alongside the configured display name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    /// Optional device label, such as "Proxmox host" or "Synology NAS".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_name: Option<String>,
    /// Remote URL or local uploaded-image path rendered as the server icon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// `local`, or an http(s) URL of a node_exporter / metrics endpoint.
    pub endpoint: String,
    /// Scrape interval in seconds (default 15).
    #[serde(default = "default_interval")]
    pub interval_secs: u64,
}

impl NodeConfig {
    pub fn interval(&self) -> Duration {
        Duration::from_secs(self.interval_secs)
    }

    pub fn is_local(&self) -> bool {
        self.endpoint == "local"
    }
}

fn default_interval() -> u64 {
    15
}

#[derive(Clone, Debug, Copy, PartialEq)]
pub struct LoadAvg {
    pub one: f64,
    pub five: f64,
    pub fifteen: f64,
}

/// Aggregated host stats, normalized between the local sysinfo source and
/// remote node_exporter sources so widgets render either identically.
#[derive(Clone, Debug, PartialEq)]
pub struct NodeStats {
    pub cpu_usage_percent: f64,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub load: Option<LoadAvg>,
    pub disk_used_bytes: u64,
    pub disk_total_bytes: u64,
    pub disk_mount: String,
    pub uptime_secs: u64,
    pub collected_at: Instant,
}

impl NodeStats {
    pub fn memory_usage_percent(&self) -> f64 {
        ratio_percent(self.memory_used_bytes, self.memory_total_bytes)
    }

    pub fn disk_usage_percent(&self) -> f64 {
        ratio_percent(self.disk_used_bytes, self.disk_total_bytes)
    }
}

fn ratio_percent(part: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        (part as f64 / total as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentages_handle_zero_total() {
        let stats = NodeStats {
            cpu_usage_percent: 0.0,
            memory_used_bytes: 5,
            memory_total_bytes: 10,
            load: None,
            disk_used_bytes: 0,
            disk_total_bytes: 0,
            disk_mount: String::new(),
            uptime_secs: 0,
            collected_at: Instant::now(),
        };
        assert_eq!(stats.memory_usage_percent(), 50.0);
        assert_eq!(stats.disk_usage_percent(), 0.0);
    }

    #[test]
    fn interval_defaults_to_15s() {
        let node: NodeConfig =
            serde_json::from_str(r#"{"id":"a","name":"A","endpoint":"local"}"#).unwrap();
        assert_eq!(node.interval(), Duration::from_secs(15));
        assert!(node.is_local());
    }
}
