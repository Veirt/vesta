pub mod assets;
pub mod manager;
pub mod manifest;
pub mod widget;

use std::sync::Arc;

use crate::plugins::manager::PluginManager;
use crate::widget_system::{WidgetHandlerDyn, WidgetRegistry};

pub use crate::plugins::widget::PLUGIN_NAME_PREFIX;

/// The registry snapshot for one rebuild: the shared built-in handlers plus
/// one handler per widget declared by each installed plugin.
pub fn build_registry(
    builtins: &[Arc<dyn WidgetHandlerDyn>],
    plugins: &PluginManager,
) -> WidgetRegistry {
    let mut registry = WidgetRegistry::new();
    for handler in builtins {
        registry = registry.register_shared(Arc::clone(handler));
    }
    for record in plugins.load_all() {
        for spec in &record.manifest.widgets {
            let handler = widget::PluginWidgetHandler::new(&record.manifest.id, spec.clone());
            registry = registry.register(handler);
        }
    }
    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Dashboard, DashboardItem, Widget};
    use serde_json::json;
    use std::path::{Path, PathBuf};

    #[tokio::test]
    async fn plugin_widgets_are_registered_and_validated() {
        let plugins = installed_fixture_manager().await;
        let registry = build_registry(&[], &plugins);

        let names: Vec<&str> = registry
            .names()
            .into_iter()
            .filter(|name| name.starts_with(widget::PLUGIN_NAME_PREFIX))
            .collect();
        assert_eq!(
            names,
            vec!["Plugin:hello-world:counter"],
            "names: {names:?}"
        );

        // The plugin descriptor exposes the manifest metadata.
        let descriptor = registry
            .catalog_descriptors()
            .into_iter()
            .find(|descriptor| descriptor.name == "Plugin:hello-world:counter")
            .unwrap();
        assert_eq!(
            descriptor.description,
            "Configured via the label field; add or subtract on click."
        );
        assert_eq!(
            (descriptor.default_width, descriptor.default_height),
            (3, 2)
        );
        assert_eq!((descriptor.min_width, descriptor.min_height), (3, 2));
        assert_eq!(
            descriptor
                .fields
                .iter()
                .map(|field| field.key.as_ref())
                .collect::<Vec<_>>(),
            vec!["start_value", "label", "theme"]
        );

        // The dashboard validator accepts a plugin tile with valid config and
        // rejects a required field left empty.
        let valid = Dashboard {
            nodes: vec![],
            items: vec![DashboardItem::Widget(Widget {
                id: "tile-1".to_string(),
                title: "Counter".to_string(),
                name: "Plugin:hello-world:counter".to_string(),
                group: None,
                config: Some(plugin_config(&json!({"label": "Count"}))),
                x: None,
                y: None,
                width: None,
                height: None,
            })],
            groups: vec![],
        };
        registry.validate_dashboard(&valid).unwrap();

        let missing = Dashboard {
            nodes: vec![],
            items: vec![DashboardItem::Widget(Widget {
                config: Some(plugin_config(&json!({}))),
                ..valid
                    .items
                    .clone()
                    .pop()
                    .and_then(|item| match item {
                        DashboardItem::Widget(widget) => Some(widget),
                        _ => None,
                    })
                    .unwrap()
            })],
            groups: vec![],
        };
        let error = registry.validate_dashboard(&missing).unwrap_err();
        assert!(error.to_string().contains("required"), "{error}");
    }

    fn plugin_config(value: &serde_json::Value) -> crate::config::WidgetConfig {
        value
            .as_object()
            .map(|object| object.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default()
    }

    async fn installed_fixture_manager() -> PluginManager {
        let root = std::env::temp_dir().join(format!(
            "vesta-shared-registry-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let manager = PluginManager::new(&root).unwrap();
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/plugins/hello-world");
        let repo = git_fixture_repo(&fixture, "registry");
        manager.install(&repo.to_string_lossy()).await.unwrap();
        std::fs::remove_dir_all(&repo).ok();
        manager
    }

    /// Like the manager tests: fixture trees become real git repositories so
    /// the installer's clone path is exercised end to end.
    fn git_fixture_repo(fixture: &Path, name: &str) -> PathBuf {
        let repo = std::env::temp_dir().join(format!(
            "vesta-plugin-fixture-{name}-{}",
            uuid::Uuid::new_v4().simple()
        ));
        copy_tree(fixture, &repo);
        for args in [
            vec!["init", "-q"],
            vec!["add", "."],
            vec![
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "commit",
                "-qm",
                "x",
            ],
        ] {
            let status = std::process::Command::new("git")
                .args(&args)
                .current_dir(&repo)
                .status()
                .unwrap();
            assert!(status.success(), "git failed: {args:?}");
        }
        repo
    }

    fn copy_tree(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).unwrap();
        for entry in std::fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let target = to.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_tree(&entry.path(), &target);
            } else {
                std::fs::copy(entry.path(), target).unwrap();
            }
        }
    }
}
