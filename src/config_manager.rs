use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use tokio::sync::{Mutex, OwnedMutexGuard, watch};

use crate::config::{AppSettings, Dashboard, Widget};
use crate::config_codec::{read_config, write_config};
use crate::config_layout::{normalize_groups, normalize_layout};
use crate::error::{VestaError, VestaResult};
use crate::validation::validate_dashboard_config;
use crate::widget_system::SharedWidgetRegistry;

#[cfg(test)]
type AfterWriteHook = Box<dyn FnOnce() + Send>;
#[cfg(test)]
type SharedAfterWriteHook = Arc<std::sync::Mutex<Option<AfterWriteHook>>>;

pub struct SettingsSnapshot {
    pub dashboard: Arc<Dashboard>,
    pub app: AppSettings,
}

/// Validates, writes, and atomically publishes the active TOML configuration.
pub struct ConfigManager {
    path: PathBuf,
    snapshot: Arc<RwLock<Arc<SettingsSnapshot>>>,
    write_gate: Arc<Mutex<()>>,
    registry: SharedWidgetRegistry,
    updates: watch::Sender<u64>,
    #[cfg(test)]
    after_write_before_publish: SharedAfterWriteHook,
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
            snapshot: Arc::new(RwLock::new(Arc::new(SettingsSnapshot {
                dashboard: Arc::new(dashboard),
                app,
            }))),
            write_gate: Arc::new(Mutex::new(())),
            registry,
            updates,
            #[cfg(test)]
            after_write_before_publish: Arc::new(std::sync::Mutex::new(None)),
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

    #[cfg(test)]
    pub(crate) fn set_after_write_before_publish_hook(&self, hook: impl FnOnce() + Send + 'static) {
        *self
            .after_write_before_publish
            .lock()
            .expect("config write test hook lock poisoned") = Some(Box::new(hook));
    }

    pub fn widget(&self, id: &str) -> Option<Widget> {
        self.snapshot().widget(id).cloned()
    }

    #[cfg(test)]
    /// Applies one validated config update with disk and snapshot publication serialized.
    pub async fn update<F>(&self, mutate: F) -> VestaResult<Arc<SettingsSnapshot>>
    where
        F: FnOnce(&SettingsSnapshot) -> VestaResult<(Dashboard, AppSettings)> + Send + 'static,
    {
        self.update_inner(None, mutate, |_| Ok(())).await
    }

    /// Also retains an external owned guard through the config transaction.
    pub async fn update_with_guard<F>(
        &self,
        guard: OwnedMutexGuard<()>,
        mutate: F,
    ) -> VestaResult<Arc<SettingsSnapshot>>
    where
        F: FnOnce(&SettingsSnapshot) -> VestaResult<(Dashboard, AppSettings)> + Send + 'static,
    {
        self.update_inner(Some(guard), mutate, |_| Ok(())).await
    }

    /// Runs a synchronous side effect in the owned writer after publishing the new snapshot.
    pub async fn update_with_post_publish<F, P>(
        &self,
        mutate: F,
        post_publish: P,
    ) -> VestaResult<Arc<SettingsSnapshot>>
    where
        F: FnOnce(&SettingsSnapshot) -> VestaResult<(Dashboard, AppSettings)> + Send + 'static,
        P: FnOnce(&SettingsSnapshot) -> VestaResult<()> + Send + 'static,
    {
        self.update_inner(None, mutate, post_publish).await
    }

    async fn update_inner<F, P>(
        &self,
        lifecycle_guard: Option<OwnedMutexGuard<()>>,
        mutate: F,
        post_publish: P,
    ) -> VestaResult<Arc<SettingsSnapshot>>
    where
        F: FnOnce(&SettingsSnapshot) -> VestaResult<(Dashboard, AppSettings)> + Send + 'static,
        P: FnOnce(&SettingsSnapshot) -> VestaResult<()> + Send + 'static,
    {
        let write_guard = Arc::clone(&self.write_gate).lock_owned().await;
        let path = self.path.clone();
        let snapshot = Arc::clone(&self.snapshot);
        let registry = self.registry.clone();
        let updates = self.updates.clone();
        #[cfg(test)]
        let after_write_before_publish = Arc::clone(&self.after_write_before_publish);

        // The owned worker retains both gates through publication and side effects.
        tokio::task::spawn_blocking(move || {
            let _write_guard = write_guard;
            let _lifecycle_guard = lifecycle_guard;
            let current = snapshot
                .read()
                .expect("settings snapshot lock poisoned")
                .clone();
            let (mut dashboard, app) = mutate(&current)?;
            let active_registry = registry.current();
            normalize_layout(&mut dashboard, &active_registry);
            normalize_groups(&mut dashboard);
            validate_dashboard_config(&dashboard, &app, &active_registry)?;
            write_config(&path, &dashboard, &app)?;

            #[cfg(test)]
            if let Some(hook) = after_write_before_publish
                .lock()
                .expect("config write test hook lock poisoned")
                .take()
            {
                hook();
            }

            let next = Arc::new(SettingsSnapshot {
                dashboard: Arc::new(dashboard),
                app,
            });
            *snapshot.write().expect("settings snapshot lock poisoned") = next.clone();
            updates.send_modify(|revision| *revision = revision.wrapping_add(1));
            post_publish(&next)?;
            Ok(next)
        })
        .await
        .map_err(|error| VestaError::Config(format!("config write task failed: {error}")))?
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

    #[tokio::test]
    async fn cancelled_update_finishes_disk_write_and_publication() {
        let path = std::env::temp_dir().join(format!(
            "vesta-cancelled-config-{}.toml",
            uuid::Uuid::new_v4().simple()
        ));
        let manager = std::sync::Arc::new(
            ConfigManager::new(&path, SharedWidgetRegistry::new(WidgetRegistry::new())).unwrap(),
        );
        let (write_done_tx, write_done_rx) = tokio::sync::oneshot::channel();
        let (publish_tx, publish_rx) = std::sync::mpsc::channel();
        manager.set_after_write_before_publish_hook(move || {
            write_done_tx.send(()).unwrap();
            publish_rx.recv().unwrap();
        });

        let update = {
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

        write_done_rx.await.unwrap();
        assert!(fs::read_to_string(&path).unwrap().contains("Updated Lab"));
        assert_ne!(
            manager.settings_snapshot().app.dashboard_title,
            "Updated Lab"
        );
        update.abort();
        assert!(matches!(update.await, Err(error) if error.is_cancelled()));
        publish_tx.send(()).unwrap();

        // This update cannot acquire the writer gate until the cancelled
        // transaction has published its snapshot and revision.
        manager
            .update(|current| {
                let mut app = current.app.clone();
                app.dashboard_title.push_str(" + second");
                Ok(((*current.dashboard).clone(), app))
            })
            .await
            .unwrap();

        let snapshot = manager.settings_snapshot();
        assert_eq!(snapshot.app.dashboard_title, "Updated Lab + second");
        assert!(
            fs::read_to_string(&path)
                .unwrap()
                .contains("Updated Lab + second")
        );
        let _ = fs::remove_file(path);
    }
}
