use std::sync::Arc;
use std::thread_local;
use std::time::Instant;

use sysinfo::{Disks, System};

use crate::metrics::node::{LoadAvg, NodeStats};

/// Collects host stats in-process via sysinfo. sysinfo accumulates CPU usage
/// between refreshes, so `System` state must persist across calls;
/// thread-local storage keeps `collect` callable from any blocking context.
#[derive(Default)]
pub struct LocalCollector;

impl LocalCollector {
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }

    pub fn collect(&self) -> Result<NodeStats, sysinfo::Error> {
        thread_local! {
            static SYSTEM: std::cell::RefCell<Option<System>> =
                const { std::cell::RefCell::new(None) };
            static DISKS: std::cell::RefCell<Option<Disks>> =
                const { std::cell::RefCell::new(None) };
        }

        let (cpu_usage_percent, memory_used_bytes, memory_total_bytes, load) =
            SYSTEM.with(|cell| {
                let mut slot = cell.borrow_mut();
                let sys = slot.get_or_insert_with(|| System::new().expect("system init"));
                sys.refresh_cpu_usage();
                sys.refresh_memory();
                let load = System::load_average()
                    .map(|l| LoadAvg {
                        one: l.one,
                        five: l.five,
                        fifteen: l.fifteen,
                    })
                    .ok();
                (
                    f64::from(sys.global_cpu_usage()),
                    sys.used_memory(),
                    sys.total_memory(),
                    load,
                )
            });

        let (disk_mount, disk_used_bytes, disk_total_bytes) =
            DISKS.with(|cell| -> (String, u64, u64) {
                let mut slot = cell.borrow_mut();
                let disks = slot.get_or_insert_with(|| {
                    Disks::new_with_refreshed_list().expect("disk enumeration")
                });
                disks.refresh(true);
                let mut found = ("/".to_string(), 0u64, 0u64);
                for disk in disks {
                    if disk.mount_point() == std::path::Path::new("/") {
                        let total = disk.total_space();
                        let used = total.saturating_sub(disk.available_space());
                        found = ("/".to_string(), used, total);
                        break;
                    }
                }
                found
            });

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
