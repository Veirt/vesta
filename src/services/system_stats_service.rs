use std::sync::Arc;
use std::time::{Duration, Instant};
use sysinfo::{Disks, System};
use tokio::sync::RwLock;
use tokio::time::interval;

#[derive(Debug, Clone, Copy)]
pub struct SystemStatsSnapshot {
    pub cpu_usage: f32,
    pub memory_usage: f64,
    pub memory_used: u64,
    pub memory_total: u64,
    pub disk_usage: f64,
    pub disk_used: u64,
    pub disk_total: u64,
    pub load_avg: f64,
    pub timestamp: Instant,
}

impl SystemStatsSnapshot {
    pub fn cpu_usage_percent(&self) -> f64 {
        self.cpu_usage as f64
    }

    pub fn memory_usage_percent(&self) -> f64 {
        self.memory_usage
    }

    pub fn disk_usage_percent(&self) -> f64 {
        self.disk_usage
    }
}

pub struct SystemStatsService {
    snapshot: RwLock<Arc<SystemStatsSnapshot>>,
}

impl SystemStatsService {
    pub fn new(refresh_interval: Duration) -> Arc<Self> {
        let initial = Arc::new(SystemStatsSnapshot {
            cpu_usage: 0.0,
            memory_usage: 0.0,
            memory_used: 0,
            memory_total: 0,
            disk_usage: 0.0,
            disk_used: 0,
            disk_total: 0,
            load_avg: 0.0,
            timestamp: Instant::now(),
        });

        let service = Arc::new(Self {
            snapshot: RwLock::new(initial),
        });

        let service_clone = Arc::clone(&service);
        tokio::spawn(async move {
            service_clone.run_background_refresh(refresh_interval).await;
        });

        service
    }

    pub async fn get_snapshot(&self) -> Arc<SystemStatsSnapshot> {
        self.snapshot.read().await.clone()
    }

    async fn refresh_now(&self) {
        let new_snapshot = Self::collect_stats();
        let mut snapshot = self.snapshot.write().await;
        *snapshot = Arc::new(new_snapshot);
    }

    async fn run_background_refresh(&self, interval_duration: Duration) {
        let mut ticker = interval(interval_duration);
        self.refresh_now().await;
        loop {
            ticker.tick().await;
            self.refresh_now().await;
        }
    }

    fn collect_stats() -> SystemStatsSnapshot {
        thread_local! {
            static SYSTEM: std::cell::RefCell<System> = std::cell::RefCell::new(System::new());
            static DISKS: std::cell::RefCell<Disks> = std::cell::RefCell::new(Disks::new_with_refreshed_list());
        }

        let (cpu_usage, memory_used, memory_total, load_avg) = SYSTEM.with(|sys| {
            let mut sys = sys.borrow_mut();
            sys.refresh_cpu_usage();
            sys.refresh_memory();
            let cpu_usage = sys.global_cpu_usage();
            let memory_used = sys.used_memory();
            let memory_total = sys.total_memory();
            let load_avg = System::load_average();
            (cpu_usage, memory_used, memory_total, load_avg)
        });

        let (disk_usage, disk_used, disk_total) = DISKS.with(|disks| {
            let mut disks = disks.borrow_mut();
            disks.refresh(false);
            let mut disk_total = 0u64;
            let mut disk_used = 0u64;
            let mut disk_usage = 0.0;
            for disk in disks.list() {
                if disk.mount_point() == std::path::Path::new("/") {
                    disk_total = disk.total_space();
                    disk_used = disk_total.saturating_sub(disk.available_space());
                    disk_usage = if disk_total > 0 {
                        (disk_used as f64 / disk_total as f64) * 100.0
                    } else {
                        0.0
                    };
                    break;
                }
            }
            (disk_usage, disk_used, disk_total)
        });

        SystemStatsSnapshot {
            cpu_usage,
            memory_usage: if memory_total > 0 {
                (memory_used as f64 / memory_total as f64) * 100.0
            } else {
                0.0
            },
            memory_used: memory_used / 1024 / 1024,
            memory_total: memory_total / 1024 / 1024,
            disk_usage,
            disk_used: disk_used / 1024 / 1024 / 1024,
            disk_total: disk_total / 1024 / 1024 / 1024,
            load_avg: load_avg.one,
            timestamp: Instant::now(),
        }
    }
}
