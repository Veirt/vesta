use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::Instant;

use reqwest::Client;

use crate::error::MetricsError;
use crate::metrics::node::{LoadAvg, NodeStats};
use prometheus_parse::{Sample, Scrape, Value};

/// Builds host stats from node_exporter Prometheus text exposition.
/// CPU percent requires two consecutive scrapes of the `node_cpu_seconds_total`
/// counter (idle seconds are differenced and divided by wall time); memory,
/// disk, load and uptime come straight from gauges.
pub struct NodeExporterCollector {
    client: Client,
    /// Last scrape's (idle, total) CPU seconds per endpoint, with wall-clock
    /// time. A Mutex is fine: one writer per endpoint, held for microseconds.
    cpu_state: Mutex<CpuState>,
}

#[derive(Default)]
struct CpuState {
    previous: HashMap<String, CpuBaseline>,
    active_endpoints: Option<HashSet<String>>,
}

#[derive(Clone, Copy)]
struct CpuBaseline {
    idle_seconds: f64,
    total_seconds: f64,
}

impl NodeExporterCollector {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            cpu_state: Mutex::new(CpuState::default()),
        }
    }

    pub(crate) fn set_active_endpoints(&self, endpoints: &HashSet<String>) {
        let mut state = self.cpu_state.lock().expect("cpu baseline lock");
        state
            .previous
            .retain(|endpoint, _| endpoints.contains(endpoint));
        state.active_endpoints = Some(endpoints.clone());
    }

    pub async fn scrape(&self, endpoint: &str) -> Result<NodeStats, MetricsError> {
        let samples = self.fetch_samples(endpoint).await?;
        self.build_stats(endpoint, &samples)
    }

    pub fn build_stats(
        &self,
        endpoint: &str,
        samples: &[Sample],
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

    /// CPU% = 1 - delta(idle) / delta(total) across scrapes, where `total`
    /// sums every mode of every core. Using both counters removes the wall
    /// clock and core count from the math, so multi-core hosts are exact.
    /// The first scrape records the baseline and reports 0.
    fn cpu_percent(&self, endpoint: &str, samples: &[Sample]) -> Result<f64, MetricsError> {
        let (idle_now, total_now) = cpu_counters(samples)?;

        let mut state = self.cpu_state.lock().expect("cpu baseline lock");
        let active = state
            .active_endpoints
            .as_ref()
            .is_none_or(|endpoints| endpoints.contains(endpoint));
        if !active {
            return Ok(0.0);
        }
        let previous = state.previous.get(endpoint).copied();
        state.previous.insert(
            endpoint.to_string(),
            CpuBaseline {
                idle_seconds: idle_now,
                total_seconds: total_now,
            },
        );
        drop(state);

        Ok(match previous {
            Some(prev) => {
                let idle_delta = (idle_now - prev.idle_seconds).max(0.0);
                let total_delta = (total_now - prev.total_seconds).max(0.0);
                if total_delta <= 0.0 {
                    0.0
                } else {
                    ((1.0 - idle_delta / total_delta) * 100.0).clamp(0.0, 100.0)
                }
            }
            None => 0.0,
        })
    }

    async fn fetch_samples(&self, endpoint: &str) -> Result<Vec<Sample>, MetricsError> {
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
        Scrape::parse(body.lines().map(|line| Ok(line.to_owned())))
            .map(|scrape| scrape.samples)
            .map_err(|e| MetricsError(format!("scrape {endpoint}: {e}")))
    }
}

fn missing(metric: &str) -> MetricsError {
    MetricsError(format!("node_exporter response missing {metric}"))
}

/// Prometheus values are f64; byte gauges fit u64 up to 2^53 exactly.
fn bytes(value: f64) -> u64 {
    value.max(0.0) as u64
}

fn numeric_value(sample: &Sample) -> Option<f64> {
    match sample.value {
        Value::Counter(value) | Value::Gauge(value) | Value::Untyped(value) => Some(value),
        Value::Histogram(_) | Value::Summary(_) => None,
    }
}

fn gauge(samples: &[Sample], name: &str) -> Option<f64> {
    samples
        .iter()
        .find(|sample| sample.metric == name)
        .and_then(numeric_value)
}

fn cpu_counters(samples: &[Sample]) -> Result<(f64, f64), MetricsError> {
    let mut idle = 0.0;
    let mut total = 0.0;
    let mut found = false;
    for sample in samples
        .iter()
        .filter(|sample| sample.metric == "node_cpu_seconds_total")
    {
        let value = numeric_value(sample).ok_or_else(|| missing("node_cpu_seconds_total"))?;
        found = true;
        total += value;
        if sample.labels.get("mode") == Some("idle") {
            idle += value;
        }
    }
    if !found {
        return Err(missing("node_cpu_seconds_total"));
    }
    Ok((idle, total))
}

struct DiskUsage {
    mount: String,
    used: f64,
    total: f64,
}

fn disk_usage(samples: &[Sample]) -> Result<DiskUsage, MetricsError> {
    let total = samples
        .iter()
        .find(|s| {
            s.metric == "node_filesystem_size_bytes"
                && s.labels.get("mountpoint") == Some("/")
                && s.labels
                    .get("fstype")
                    .is_some_and(|f| f != "tmpfs" && f != "squashfs")
        })
        .ok_or_else(|| missing("node_filesystem_size_bytes"))?;

    let mount = total
        .labels
        .get("mountpoint")
        .map(str::to_owned)
        .unwrap_or_else(|| "/".to_string());

    let avail = samples
        .iter()
        .find(|s| {
            s.metric == "node_filesystem_avail_bytes"
                && s.labels.get("mountpoint") == total.labels.get("mountpoint")
        })
        .and_then(numeric_value)
        .unwrap_or(0.0);

    Ok(DiskUsage {
        mount,
        used: (numeric_value(total).unwrap_or(0.0) - avail).max(0.0),
        total: numeric_value(total).unwrap_or(0.0),
    })
}

fn boot_time_to_uptime(boot_time_secs: f64) -> u64 {
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    now_secs.saturating_sub(boot_time_secs as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cpu_samples(idle: f64, user: f64) -> Vec<Sample> {
        let exposition = [
            format!("node_cpu_seconds_total{{cpu=\"0\",mode=\"idle\"}} {idle}"),
            format!("node_cpu_seconds_total{{cpu=\"0\",mode=\"user\"}} {user}"),
        ];
        Scrape::parse(exposition.into_iter().map(Ok))
            .unwrap()
            .samples
    }

    #[test]
    fn disk_usage_selects_root_even_when_boot_is_listed_first() {
        let exposition = [
            "# TYPE node_filesystem_size_bytes gauge",
            "node_filesystem_size_bytes{device=\"/dev/sda1\",fstype=\"ext4\",mountpoint=\"/boot\"} 1000",
            "node_filesystem_avail_bytes{device=\"/dev/sda1\",fstype=\"ext4\",mountpoint=\"/boot\"} 400",
            "node_filesystem_size_bytes{device=\"/dev/mapper/root\",fstype=\"ext4\",mountpoint=\"/\"} 10000",
            "node_filesystem_avail_bytes{device=\"/dev/mapper/root\",fstype=\"ext4\",mountpoint=\"/\"} 2500",
        ];
        let samples = Scrape::parse(exposition.into_iter().map(|line| Ok(line.to_string())))
            .unwrap()
            .samples;

        let disk = disk_usage(&samples).unwrap();

        assert_eq!(disk.mount, "/");
        assert_eq!(disk.total, 10_000.0);
        assert_eq!(disk.used, 7_500.0);
    }

    #[test]
    fn endpoint_pruning_keeps_active_cpu_baselines_and_resets_removed_ones() {
        let collector = NodeExporterCollector::new(Client::new());
        assert_eq!(
            collector
                .cpu_percent("active", &cpu_samples(100.0, 100.0))
                .unwrap(),
            0.0
        );
        assert_eq!(
            collector
                .cpu_percent("removed", &cpu_samples(100.0, 100.0))
                .unwrap(),
            0.0
        );

        collector.set_active_endpoints(&HashSet::from(["active".to_string()]));

        assert_eq!(
            collector
                .cpu_percent("active", &cpu_samples(105.0, 105.0))
                .unwrap(),
            50.0
        );
        assert_eq!(
            collector
                .cpu_percent("removed", &cpu_samples(105.0, 105.0))
                .unwrap(),
            0.0
        );
    }
}
