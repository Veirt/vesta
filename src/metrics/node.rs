use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NodeConfig {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    pub endpoint: String,
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

    pub fn to_json(&self, include_age: bool) -> serde_json::Value {
        let mut value = serde_json::json!({
            "cpu_usage_percent": self.cpu_usage_percent,
            "memory": {
                "used_bytes": self.memory_used_bytes,
                "total_bytes": self.memory_total_bytes,
                "usage_percent": self.memory_usage_percent(),
            },
            "disk": {
                "mount": self.disk_mount,
                "used_bytes": self.disk_used_bytes,
                "total_bytes": self.disk_total_bytes,
                "usage_percent": self.disk_usage_percent(),
            },
            "load": self.load.map(|load| serde_json::json!({
                "one": load.one,
                "five": load.five,
                "fifteen": load.fifteen,
            })),
            "uptime_secs": self.uptime_secs,
        });
        if include_age && let Some(obj) = value.as_object_mut() {
            obj.insert(
                "collected_age_secs".to_string(),
                serde_json::json!(self.collected_at.elapsed().as_secs()),
            );
        }
        value
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
