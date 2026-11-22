use std::sync::{Arc, Mutex};
use std::time::Instant;

use sysinfo::{Disks, System};

use crate::metrics::node::{LoadAvg, NodeStats};

struct LocalState {
    system: System,
    disks: Disks,
}

pub struct LocalCollector {
    state: Mutex<Option<LocalState>>,
}

impl Default for LocalCollector {
    fn default() -> Self {
        Self {
            state: Mutex::new(None),
        }
    }
}

impl LocalCollector {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn collect(&self) -> Result<NodeStats, sysinfo::Error> {
        let mut guard = self.state.lock().expect("local collector lock poisoned");
        let state = match guard.as_mut() {
            Some(state) => state,
            None => {
                let initial = LocalState {
                    system: System::new()?,
                    disks: Disks::new_with_refreshed_list()?,
                };
                guard.insert(initial)
            }
        };

        state.system.refresh_cpu_usage();
        state.system.refresh_memory();
        let cpu_usage_percent = f64::from(state.system.global_cpu_usage());
        let memory_used_bytes = state.system.used_memory();
        let memory_total_bytes = state.system.total_memory();
        let load = System::load_average()
            .map(|l| LoadAvg {
                one: l.one,
                five: l.five,
                fifteen: l.fifteen,
            })
            .ok();

        state.disks.refresh(true);
        let mut disk_mount = "/".to_string();
        let mut disk_used_bytes = 0u64;
        let mut disk_total_bytes = 0u64;
        for disk in &state.disks {
            if disk.mount_point() == std::path::Path::new("/") {
                disk_total_bytes = disk.total_space();
                disk_used_bytes = disk_total_bytes.saturating_sub(disk.available_space());
                disk_mount = "/".to_string();
                break;
            }
        }

        Ok(NodeStats {
            cpu_usage_percent,
            memory_used_bytes,
            memory_total_bytes,
            load,
            disk_mount,
            disk_used_bytes,
            disk_total_bytes,
            uptime_secs: System::uptime().unwrap_or(0),
            collected_at: Instant::now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_local_stats() {
        let collector = LocalCollector::new();
        let stats = collector.collect().unwrap();
        assert!(stats.memory_usage_percent() <= 100.0);
        assert!(stats.disk_usage_percent() <= 100.0);
    }
}
