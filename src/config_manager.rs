use serde::Serialize;
use std::sync::{Arc, RwLock, RwLockReadGuard};
use tokio::sync::broadcast;

use crate::{
    config::{Dashboard, load_config},
    error::{VestaError, VestaResult},
    widget_system::WidgetRegistry,
};

#[derive(Debug, Clone)]
pub struct ConfigChangeEvent {
    pub timestamp: std::time::SystemTime,
    pub changes: ConfigChanges,
}

#[derive(Debug, Clone)]
pub enum ConfigChanges {
    Reloaded,
    ServicesUpdated,
    WidgetsUpdated,
}

/// Configuration manager that handles config access, validation, and change notifications
pub struct ConfigManager {
    config: Arc<RwLock<Dashboard>>,
    config_path: String,
    change_notifier: broadcast::Sender<ConfigChangeEvent>,
    widget_registry: Arc<WidgetRegistry>,
}

impl ConfigManager {
    pub fn new(config_path: &str, widget_registry: Arc<WidgetRegistry>) -> VestaResult<Self> {
        let config = load_config(config_path)?;

        widget_registry.validate_widgets(&config)?;

        let (change_notifier, _) = broadcast::channel(100);

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            config_path: config_path.to_string(),
            change_notifier,
            widget_registry,
        })
    }

    pub fn read_config(&self) -> Result<RwLockReadGuard<'_, Dashboard>, VestaError> {
        self.config
            .read()
            .map_err(|e| VestaError::Internal(format!("Failed to acquire read lock: {}", e)))
    }

    pub fn get_config(&self) -> VestaResult<Dashboard> {
        let config = self.read_config()?;
        Ok(config.clone())
    }

    pub fn reload_config(&self) -> VestaResult<()> {
        let new_config = load_config(&self.config_path)?;

        self.widget_registry.validate_widgets(&new_config)?;

        {
            let mut config = self.config.write().map_err(|e| {
                VestaError::Internal(format!("Failed to acquire write lock: {}", e))
            })?;
            *config = new_config;
        }

        let _ = self.change_notifier.send(ConfigChangeEvent {
            timestamp: std::time::SystemTime::now(),
            changes: ConfigChanges::Reloaded,
        });

        Ok(())
    }

    pub fn subscribe_to_changes(&self) -> broadcast::Receiver<ConfigChangeEvent> {
        self.change_notifier.subscribe()
    }

    pub fn get_service(
        &self,
        group: &str,
        title: &str,
    ) -> VestaResult<Option<crate::config::Service>> {
        let config = self.read_config()?;
        Ok(config.get_service(group, title).cloned())
    }

    pub fn get_widget(
        &self,
        group: &str,
        title: &str,
    ) -> VestaResult<Option<crate::config::Widget>> {
        let config = self.read_config()?;
        Ok(config.get_widget(group, title).cloned())
    }

    pub fn validate_config(&self) -> VestaResult<()> {
        let config = self.read_config()?;
        self.widget_registry.validate_widgets(&config)
    }

    pub fn get_config_stats(&self) -> VestaResult<ConfigStats> {
        let config = self.read_config()?;

        let total_services = config.groups.values().map(|g| g.services.len()).sum();
        let services_with_ping = config
            .groups
            .values()
            .flat_map(|g| &g.services)
            .filter(|s| s.ping.is_some())
            .count();
        let services_with_widgets = config
            .groups
            .values()
            .flat_map(|g| &g.services)
            .filter(|s| s.widget.is_some())
            .count();

        Ok(ConfigStats {
            total_groups: config.groups.len(),
            total_services,
            services_with_ping,
            services_with_widgets,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigStats {
    pub total_groups: usize,
    pub total_services: usize,
    pub services_with_ping: usize,
    pub services_with_widgets: usize,
}
