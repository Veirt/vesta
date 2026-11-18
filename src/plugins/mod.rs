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
pub(crate) mod test_fixtures {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    pub(crate) struct TempDir(PathBuf);

    impl TempDir {
        pub(crate) fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    pub(crate) fn temp_dir(prefix: &str) -> TempDir {
        let dir = std::env::temp_dir().join(format!("{prefix}-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }

    pub(crate) fn git(args: &[&str], dir: &Path) {
        let status = Command::new("git")
            .args(args)
            .current_dir(dir)
            .status()
            .expect("git binary available for tests");
        assert!(status.success(), "git {args:?} failed");
    }

    pub(crate) fn fixture_dir(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/plugins")
            .join(name)
    }

    /// Create a local git repository from fixture files (only tests build the
    /// fixture with the git CLI; runtime cloning is pure gix).
    pub(crate) fn git_fixture_repo(fixture: &Path, name: &str) -> TempDir {
        let repo = temp_dir(&format!("vesta-plugin-fixture-{name}"));
        copy_tree(fixture, repo.path());
        git(&["init", "-q"], repo.path());
        git(&["add", "."], repo.path());
        git(
            &[
                "-c",
                "user.name=test",
                "-c",
                "user.email=test@test",
                "commit",
                "-qm",
                "fixture",
            ],
            repo.path(),
        );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Dashboard, DashboardItem, Widget};
    use crate::widgets::weather_widget::WeatherWidget;
    use serde_json::json;

    #[tokio::test]
    async fn real_widget_descriptors_and_layout_match_shared_contract_fixture() {
        let (_root, plugins) = installed_fixture_manager().await;
        let builtins = vec![WidgetRegistry::handler(WeatherWidget::new(
            reqwest::Client::new(),
        ))];
        let registry = build_registry(&builtins, &plugins);
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/widget-contract.json"))
                .unwrap();

        let descriptors = registry.catalog_descriptors();
        assert_eq!(
            serde_json::to_value(&descriptors).unwrap(),
            fixture["descriptors"]
        );

        for case in fixture["layout"]["service_minimums"].as_array().unwrap() {
            let group_span = case["group_span"].as_u64().unwrap() as u8;
            let minimum = (
                case["width"].as_u64().unwrap() as u8,
                case["height"].as_u64().unwrap() as u8,
            );
            assert_eq!(crate::config_layout::service_min_size(group_span), minimum);
        }

        for case in fixture["layout"]["widget_minimums"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let minimum = (
                case["width"].as_u64().unwrap() as u8,
                case["height"].as_u64().unwrap() as u8,
            );
            assert_eq!(registry.min_size(name), Some(minimum));
        }
    }

    #[tokio::test]
    async fn plugin_widgets_are_registered_and_validated() {
        let (_root, plugins) = installed_fixture_manager().await;
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

    async fn installed_fixture_manager() -> (test_fixtures::TempDir, PluginManager) {
        let root = test_fixtures::temp_dir("vesta-shared-registry");
        let manager = PluginManager::new(root.path()).unwrap();
        let fixture = test_fixtures::fixture_dir("hello-world");
        let repo = test_fixtures::git_fixture_repo(&fixture, "registry");
        manager
            .install(&repo.path().to_string_lossy())
            .await
            .unwrap();
        (root, manager)
    }
}
