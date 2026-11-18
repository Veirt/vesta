use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use tokio::sync::{Mutex, watch};

use crate::config::{AppSettings, Dashboard, Widget};
use crate::config_codec::{read_config, write_config};
use crate::config_layout::{normalize_groups, normalize_layout};
use crate::error::{VestaError, VestaResult};
use crate::validation::validate_dashboard_config;
use crate::widget_system::SharedWidgetRegistry;

pub struct SettingsSnapshot {
    pub dashboard: Arc<Dashboard>,
    pub app: AppSettings,
}

/// Validates, writes, and atomically publishes the active TOML configuration.
pub struct ConfigManager {
    path: PathBuf,
    snapshot: RwLock<Arc<SettingsSnapshot>>,
    write_gate: Mutex<()>,
    registry: SharedWidgetRegistry,
    updates: watch::Sender<u64>,
}

impl ConfigManager {
    pub fn new(path: impl Into<PathBuf>, registry: SharedWidgetRegistry) -> VestaResult<Self> {
        let path = path.into();
        let config = read_config(&path)?;
        let active_registry = registry.current();
        let mut dashboard = config.dashboard();
        normalize_layout(&mut dashboard, &active_registry);
        normalize_groups(&mut dashboard);
        let app = config.app;
        validate_dashboard_config(&dashboard, &app, &active_registry)?;
        let (updates, _) = watch::channel(0);

        Ok(Self {
            path,
            snapshot: RwLock::new(Arc::new(SettingsSnapshot {
                dashboard: Arc::new(dashboard),
                app,
            })),
            write_gate: Mutex::new(()),
            registry,
            updates,
        })
    }

    pub fn snapshot(&self) -> Arc<Dashboard> {
        self.snapshot
            .read()
            .expect("settings snapshot lock poisoned")
            .dashboard
            .clone()
    }

    pub fn settings_snapshot(&self) -> Arc<SettingsSnapshot> {
        self.snapshot
            .read()
            .expect("settings snapshot lock poisoned")
            .clone()
    }

    pub fn subscribe_updates(&self) -> watch::Receiver<u64> {
        self.updates.subscribe()
    }

    pub fn widget(&self, id: &str) -> Option<Widget> {
        self.snapshot().widget(id).cloned()
    }

    /// Atomically reads the latest snapshot, applies `mutate`, normalizes and
    /// validates the result, writes it atomically to disk on a blocking thread,
    /// and publishes the new snapshot. Holding `write_gate` across the full
    /// read-modify-write cycle prevents concurrent saves from overwriting each
    /// other.
    pub async fn update<F>(&self, mutate: F) -> VestaResult<Arc<SettingsSnapshot>>
    where
        F: FnOnce(&SettingsSnapshot) -> VestaResult<(Dashboard, AppSettings)>,
    {
        let _write = self.write_gate.lock().await;
        let current = self.settings_snapshot();
        let (mut dashboard, app) = mutate(&current)?;
        let registry = self.registry.current();
        normalize_layout(&mut dashboard, &registry);
        normalize_groups(&mut dashboard);
        validate_dashboard_config(&dashboard, &app, &registry)?;

        let path = self.path.clone();
        let dashboard_to_write = dashboard.clone();
        let app_to_write = app.clone();
        tokio::task::spawn_blocking(move || {
            write_config(&path, &dashboard_to_write, &app_to_write)
        })
        .await
        .map_err(|error| VestaError::Config(format!("config write task failed: {error}")))??;

        let next = Arc::new(SettingsSnapshot {
            dashboard: Arc::new(dashboard),
            app,
        });
        *self
            .snapshot
            .write()
            .expect("settings snapshot lock poisoned") = next.clone();
        self.updates
            .send_modify(|revision| *revision = revision.wrapping_add(1));
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::config::{Dashboard, DashboardItem, Service};
    use crate::widget_system::{SharedWidgetRegistry, WidgetRegistry};

    use super::ConfigManager;

    #[tokio::test]
    async fn concurrent_updates_do_not_clobber_disjoint_slices() {
        let path = std::env::temp_dir().join(format!(
            "vesta-concurrent-config-{}.toml",
            uuid::Uuid::new_v4().simple()
        ));
        let manager = std::sync::Arc::new(
            ConfigManager::new(&path, SharedWidgetRegistry::new(WidgetRegistry::new())).unwrap(),
        );

        let a = {
            let manager = std::sync::Arc::clone(&manager);
            tokio::spawn(async move {
                manager
                    .update(|current| {
                        let mut app = current.app.clone();
                        app.dashboard_title = "Updated Lab".to_string();
                        Ok(((*current.dashboard).clone(), app))
                    })
                    .await
            })
        };
        let b = {
            let manager = std::sync::Arc::clone(&manager);
            tokio::spawn(async move {
                manager
                    .update(|current| {
                        let dashboard = Dashboard {
                            items: vec![DashboardItem::Service(Service {
                                id: "svc-1".to_string(),
                                title: "Service".to_string(),
                                group: None,
                                icon: None,
                                href: None,
                                show_destination: true,
                                x: None,
                                y: None,
                                width: None,
                                height: None,
                                ping: None,
                            })],
                            ..(*current.dashboard).clone()
                        };
                        Ok((dashboard, current.app.clone()))
                    })
                    .await
            })
        };

        a.await.unwrap().unwrap();
        b.await.unwrap().unwrap();
        let snapshot = manager.settings_snapshot();
        assert_eq!(snapshot.app.dashboard_title, "Updated Lab");
        assert_eq!(snapshot.dashboard.items.len(), 1);
        let _ = fs::remove_file(path);
    }
}
