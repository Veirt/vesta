use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

use reqwest::Client;

use crate::error::MetricsError;
use crate::metrics::node::{LoadAvg, NodeStats};
use crate::metrics::parser::{MetricSample, parse_exposition};

/// Builds host stats from node_exporter Prometheus text exposition.
/// CPU percent requires two consecutive scrapes of the `node_cpu_seconds_total`
/// counter (idle seconds are differenced and divided by wall time); memory,
/// disk, load and uptime come straight from gauges.
pub struct NodeExporterCollector {
    client: Client,
    /// Last scrape's idle counter per endpoint, with wall-clock time. A Mutex
    /// is fine: one writer per endpoint, held for microseconds.
    previous_idle: Mutex<HashMap<String, (f64, Instant)>>,
}

impl NodeExporterCollector {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            previous_idle: Mutex::new(HashMap::new()),
        }
    }

    pub async fn scrape(&self, endpoint: &str) -> Result<NodeStats, MetricsError> {
        let samples = self.fetch_samples(endpoint).await?;
        self.build_stats(endpoint, &samples)
    }

    pub fn build_stats(
        &self,
        endpoint: &str,
        samples: &[MetricSample],
    ) -> Result<NodeStats, MetricsError> {
        let cpu = self.cpu_percent(endpoint, samples)?;
        let memory_available = gauge(samples, "node_memory_MemAvailable_bytes")
            .or_else(|| gauge(samples, "node_memory_MemFree_bytes"))
            .ok_or_else(|| missing("node_memory_MemAvailable_bytes"))?;
        let memory_total_bytes = gauge(samples, "node_memory_MemTotal_bytes")
            .ok_or_else(|| missing("node_memory_MemTotal_bytes"))?;

        let disk = disk_usage(samples)?;

        Ok(NodeStats {
            cpu_usage_percent: cpu,
            memory_used_bytes: bytes(memory_total_bytes).saturating_sub(bytes(memory_available)),
            memory_total_bytes: bytes(memory_total_bytes),
            load: Some(LoadAvg {
                one: gauge(samples, "node_load1").unwrap_or(0.0),
                five: gauge(samples, "node_load5").unwrap_or(0.0),
                fifteen: gauge(samples, "node_load15").unwrap_or(0.0),
            }),
            disk_mount: disk.mount,
            disk_used_bytes: bytes(disk.used),
            disk_total_bytes: bytes(disk.total),
            uptime_secs: gauge(samples, "node_boot_time_seconds")
                .map(boot_time_to_uptime)
                .unwrap_or(0),
            collected_at: Instant::now(),
        })
    }

    /// CPU% = delta(idle) / delta(wall clock) across scrapes. The first scrape
    /// after (re)start yields no sample and records the baseline instead.
    fn cpu_percent(&self, endpoint: &str, samples: &[MetricSample]) -> Result<f64, MetricsError> {
        let idle_now = total_idle_seconds(samples)?;
        let now = Instant::now();

        let mut baselines = self.previous_idle.lock().expect("idle baseline lock");
        let previous = baselines.get(endpoint).copied();
        baselines.insert(endpoint.to_string(), (idle_now, now));
        drop(baselines);

        Ok(match previous {
            Some((idle_prev, at_prev)) => {
                let elapsed = now.duration_since(at_prev).as_secs_f64();
                if elapsed <= 0.0 {
                    0.0
                } else {
                    (((idle_now - idle_prev) / elapsed) * 100.0).clamp(0.0, 100.0)
                }
            }
            None => 0.0,
        })
    }

    async fn fetch_samples(&self, endpoint: &str) -> Result<Vec<MetricSample>, MetricsError> {
        let response = self
            .client
            .get(endpoint)
            .send()
            .await
            .map_err(|e| MetricsError(format!("scrape {endpoint}: {e}")))?;
        if !response.status().is_success() {
            return Err(MetricsError(format!(
                "scrape {endpoint}: HTTP {}",
                response.status()
            )));
        }
        let body = response
            .text()
            .await
            .map_err(|e| MetricsError(format!("scrape {endpoint}: {e}")))?;
        parse_exposition(&body).map_err(|e| MetricsError(format!("scrape {endpoint}: {e}")))
    }
}

fn missing(metric: &str) -> MetricsError {
    MetricsError(format!("node_exporter response missing {metric}"))
}

/// Prometheus values are f64; byte gauges fit u64 up to 2^53 exactly.
fn bytes(value: f64) -> u64 {
    value.max(0.0) as u64
}

fn gauge(samples: &[MetricSample], name: &str) -> Option<f64> {
    samples.iter().find(|s| s.name == name).map(|s| s.value)
}

fn total_idle_seconds(samples: &[MetricSample]) -> Result<f64, MetricsError> {
    let has_cpu_metric = samples.iter().any(|s| s.name == "node_cpu_seconds_total");
    let idle: f64 = samples
        .iter()
        .filter(|s| {
            s.name == "node_cpu_seconds_total" && s.labels.get("mode").is_some_and(|m| m == "idle")
        })
        .map(|s| s.value)
        .sum();
    if !has_cpu_metric {
        return Err(missing("node_cpu_seconds_total"));
    }
    Ok(idle)
}

struct DiskUsage {
    mount: String,
    used: f64,
    total: f64,
}

fn disk_usage(samples: &[MetricSample]) -> Result<DiskUsage, MetricsError> {
    let total = samples
        .iter()
        .find(|s| {
            s.name == "node_filesystem_size_bytes"
                && s.labels
                    .get("fstype")
                    .is_some_and(|f| f != "tmpfs" && f != "squashfs")
        })
        .ok_or_else(|| missing("node_filesystem_size_bytes"))?;

    let mount = total
        .labels
        .get("mountpoint")
        .cloned()
        .unwrap_or_else(|| "/".to_string());

    let avail = samples
        .iter()
        .find(|s| {
            s.name == "node_filesystem_avail_bytes"
                && s.labels.get("mountpoint") == total.labels.get("mountpoint")
        })
        .map(|s| s.value)
        .unwrap_or(0.0);

    Ok(DiskUsage {
        mount,
        used: (total.value - avail).max(0.0),
        total: total.value,
    })
}

fn boot_time_to_uptime(boot_time_secs: f64) -> u64 {
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    now_secs.saturating_sub(boot_time_secs as u64)
}
