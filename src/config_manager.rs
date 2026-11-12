use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use crate::config::{Dashboard, Widget, load_config};
use crate::error::VestaResult;

/// Loads `vesta.toml` and hands out consistent snapshots. Reload re-reads from
/// disk and validates through the widget registry before swapping.
pub struct ConfigManager {
    config: RwLock<Arc<Dashboard>>,
    path: PathBuf,
    registry: Arc<crate::widget_system::WidgetRegistry>,
}

impl ConfigManager {
    pub fn new(
        path: impl Into<PathBuf>,
        registry: Arc<crate::widget_system::WidgetRegistry>,
    ) -> VestaResult<Self> {
        let path = path.into();
        let config = Arc::new(load_config(&path)?);
        registry.validate_dashboard(&config)?;
        Ok(Self {
            config: RwLock::new(config),
            path,
            registry,
        })
    }

    /// Cheap consistent snapshot; cloning the Arc is the whole cost.
    pub fn snapshot(&self) -> Arc<Dashboard> {
        self.config.read().expect("config lock poisoned").clone()
    }

    pub fn widget(&self, group: &str, title: &str) -> VestaResult<Option<Widget>> {
        Ok(self.snapshot().widget(group, title).cloned())
    }

    /// Re-read from disk, validate, then swap atomically.
    pub fn reload(&self) -> VestaResult<()> {
        let fresh = load_config(&self.path)?;
        self.registry.validate_dashboard(&fresh)?;
        crate::metrics::service::validate_nodes(&fresh.nodes)?;
        *self.config.write().expect("config lock poisoned") = Arc::new(fresh);
        tracing::info!(path = %self.path.display(), "configuration reloaded");
        Ok(())
    }
}
