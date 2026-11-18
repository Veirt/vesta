use std::path::Path;

use serde::Deserialize;

use crate::error::{VestaError, VestaResult};

/// Maximum manifest size guard: a maliciously large manifest is rejected
/// before parsing rather than read into memory unchecked.
const MAX_MANIFEST_BYTES: u64 = 256 * 1024;

pub const MANIFEST_FILE: &str = "vesta-plugin.toml";
pub const SOURCE_FILE: &str = "vesta-plugin-source.toml";

/// Static widget contributed by a plugin. Rendered by the dashboard as a
/// sandboxed iframe pointing at `entry`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct PluginWidgetSpec {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    pub entry: String,
    #[serde(default = "default_width")]
    pub width: u8,
    #[serde(default = "default_height")]
    pub height: u8,
    #[serde(default = "default_min_width")]
    pub min_width: u8,
    #[serde(default = "default_min_height")]
    pub min_height: u8,
    #[serde(default)]
    pub refresh_interval_secs: Option<u64>,
    #[serde(default)]
    pub fields: Vec<PluginFieldSpec>,
}

fn default_width() -> u8 {
    4
}

fn default_height() -> u8 {
    3
}

fn default_min_width() -> u8 {
    3
}

fn default_min_height() -> u8 {
    2
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct PluginFieldSpec {
    pub key: String,
    pub label: String,
    #[serde(default = "default_kind")]
    pub kind: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    #[serde(default)]
    pub options: Vec<PluginOptionSpec>,
}

fn default_kind() -> String {
    "text".to_string()
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct PluginOptionSpec {
    pub value: String,
    pub label: String,
}

/// Metadata persisted next to each installed plugin recording where it came
/// from and what was installed.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PluginSource {
    pub source_url: String,
    pub commit: String,
    pub installed_at: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PluginRecord {
    pub manifest: PluginManifest,
    pub source: PluginSource,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default, rename = "widgets")]
    pub widgets: Vec<PluginWidgetSpec>,
}

pub fn is_valid_slug(value: &str) -> bool {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) if first.is_ascii_lowercase() || first.is_ascii_digit() => {}
        _ => return false,
    }
    let len = value.len();
    len <= 64
        && value
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

const FIELD_KINDS: [&str; 5] = ["text", "url", "password", "number", "select"];

const MIN_TILE_SIZE: u8 = 2;
const MAX_TILE_SIZE: u8 = 12;
const MIN_PLUGIN_MIN_HEIGHT: u8 = 2;

fn validate_widget(widget: &PluginWidgetSpec, plugin_root: &Path) -> VestaResult<()> {
    check_slug(&widget.id, "widget id")?;
    if widget.title.trim().is_empty() {
        return Err(manifest_error(format!(
            "widget '{}': title must not be empty",
            widget.id
        )));
    }
    let entry_ok = is_safe_entry(&widget.entry);
    if !entry_ok {
        return Err(manifest_error(format!(
            "widget '{}': entry must be a relative path to an .html file inside the plugin",
            widget.id
        )));
    }
    if !plugin_root.join(&widget.entry).is_file() {
        return Err(manifest_error(format!(
            "widget '{}': entry '{}' does not exist in the plugin repository",
            widget.id, widget.entry
        )));
    }
    if widget.width < MIN_TILE_SIZE
        || widget.width > MAX_TILE_SIZE
        || widget.height < MIN_TILE_SIZE
        || widget.height > MAX_TILE_SIZE
    {
        return Err(manifest_error(format!(
            "widget '{}': width and height must be between {MIN_TILE_SIZE} and {MAX_TILE_SIZE}",
            widget.id
        )));
    }
    if widget.min_width < MIN_TILE_SIZE || widget.min_width > widget.width {
        return Err(manifest_error(format!(
            "widget '{}': min_width must be between {MIN_TILE_SIZE} and the widget width",
            widget.id
        )));
    }
    if widget.min_height < MIN_PLUGIN_MIN_HEIGHT || widget.min_height > widget.height {
        return Err(manifest_error(format!(
            "widget '{}': min_height must be between {MIN_PLUGIN_MIN_HEIGHT} and the widget height",
            widget.id
        )));
    }
    if widget.fields.len() > 32 {
        return Err(manifest_error(format!(
            "widget '{}': more than 32 fields",
            widget.id
        )));
    }
    for field in &widget.fields {
        validate_field(&widget.id, field)?;
    }
    Ok(())
}

fn validate_field(widget_id: &str, field: &PluginFieldSpec) -> VestaResult<()> {
    if field.key.is_empty() {
        return Err(manifest_error(format!(
            "widget '{widget_id}': a field has an empty key"
        )));
    }
    if !field
        .key
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(manifest_error(format!(
            "widget '{widget_id}': field key '{}' may only contain letters, digits and _",
            field.key
        )));
    }
    if field.label.trim().is_empty() {
        return Err(manifest_error(format!(
            "widget '{widget_id}': field '{}' must have a label",
            field.key
        )));
    }
    if !FIELD_KINDS.contains(&field.kind.as_str()) {
        return Err(manifest_error(format!(
            "widget '{widget_id}': field '{}' has unknown kind '{}' (allowed: {})",
            field.key,
            field.kind,
            FIELD_KINDS.join(", ")
        )));
    }
    if field.kind != "select" && !field.options.is_empty() {
        return Err(manifest_error(format!(
            "widget '{widget_id}': field '{}' may only declare options for kind 'select'",
            field.key
        )));
    }
    if field.kind == "select" && field.options.is_empty() {
        return Err(manifest_error(format!(
            "widget '{widget_id}': select field '{}' must declare at least one option",
            field.key
        )));
    }
    for option in &field.options {
        if option.value.is_empty() {
            return Err(manifest_error(format!(
                "widget '{widget_id}': select field '{}' has an option with an empty value",
                field.key
            )));
        }
    }
    Ok(())
}

fn check_slug(value: &str, what: &str) -> VestaResult<()> {
    if is_valid_slug(value) {
        Ok(())
    } else {
        Err(manifest_error(format!(
            "{what} '{value}' is invalid: use 1-64 lowercase letters, digits, '-' or '_'"
        )))
    }
}

fn manifest_error(message: String) -> VestaError {
    VestaError::InvalidWidgetConfig(format!("plugin manifest: {message}"))
}

/// Entry paths must stay inside the plugin directory: relative, no parent
/// components, and end with `.html` (the iframe host page kind).
fn is_safe_entry(entry: &str) -> bool {
    if entry.is_empty() || !entry.ends_with(".html") {
        return false;
    }
    let path = Path::new(entry);
    if path.is_absolute() {
        return false;
    }
    path.components().all(|component| {
        matches!(
            component,
            std::path::Component::Normal(_) | std::path::Component::CurDir
        )
    })
}

impl PluginManifest {
    /// Parse and fully validate a manifest text against the cloned plugin
    /// directory (entry files must exist).
    pub fn parse(text: &str, plugin_root: &Path) -> VestaResult<Self> {
        let manifest: PluginManifest = toml::from_str(text).map_err(|error| {
            manifest_error(format!("{MANIFEST_FILE} could not be parsed: {error}"))
        })?;
        manifest.validate(plugin_root)?;
        Ok(manifest)
    }

    pub fn validate(&self, plugin_root: &Path) -> VestaResult<()> {
        check_slug(&self.id, "plugin id")?;
        if self.name.trim().is_empty() {
            return Err(manifest_error("name must not be empty".to_string()));
        }
        if self.widgets.len() > 32 {
            return Err(manifest_error("more than 32 widgets".to_string()));
        }
        let mut seen = std::collections::HashSet::new();
        for widget in &self.widgets {
            if !seen.insert(widget.id.clone()) {
                return Err(manifest_error(format!(
                    "widget id '{}' is declared twice",
                    widget.id
                )));
            }
            validate_widget(widget, plugin_root)?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub fn widget(&self, widget_id: &str) -> Option<&PluginWidgetSpec> {
        self.widgets.iter().find(|widget| widget.id == widget_id)
    }
}

impl PluginSource {
    pub fn to_toml(&self) -> VestaResult<String> {
        toml::to_string_pretty(self).map_err(|error| {
            VestaError::InvalidWidgetConfig(format!("plugin source metadata: {error}"))
        })
    }

    pub fn from_toml(text: &str) -> VestaResult<Self> {
        toml::from_str(text).map_err(|error| {
            VestaError::InvalidWidgetConfig(format!("plugin source metadata: {error}"))
        })
    }
}

/// Read and validate `vesta-plugin.toml` from a checked-out plugin tree.
pub fn load_manifest(plugin_root: &Path) -> VestaResult<PluginManifest> {
    use std::io::Read;

    let path = plugin_root.join(MANIFEST_FILE);
    let file = std::fs::File::open(&path).map_err(|error| {
        manifest_error(format!(
            "missing or unreadable {MANIFEST_FILE} in {}: {error}",
            plugin_root.display()
        ))
    })?;
    let mut bytes = Vec::new();
    file.take(MAX_MANIFEST_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| {
            manifest_error(format!(
                "missing or unreadable {MANIFEST_FILE} in {}: {error}",
                plugin_root.display()
            ))
        })?;
    if bytes.len() as u64 > MAX_MANIFEST_BYTES {
        return Err(manifest_error(format!(
            "{MANIFEST_FILE} exceeds the size limit"
        )));
    }
    let text = String::from_utf8(bytes)
        .map_err(|error| manifest_error(format!("{MANIFEST_FILE} is not valid UTF-8: {error}")))?;
    PluginManifest::parse(&text, plugin_root)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_plugin(files: &[(&str, &str)]) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "vesta-manifest-test-{}",
            uuid::Uuid::new_v4().simple()
        ));
        for (path, contents) in files {
            let full = root.join(path);
            if let Some(parent) = full.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(full, contents).unwrap();
        }
        root
    }

    fn root_for(entry: &str) -> std::path::PathBuf {
        write_plugin(&[(entry, "<html></html>")])
    }

    #[test]
    fn parses_valid_manifest_with_defaults() {
        let root = root_for("index.html");
        let text = r#"
id = "hello-world"
name = "Hello World"

[[widgets]]
id = "counter"
title = "Counter"
entry = "index.html"
"#;
        let manifest = PluginManifest::parse(text, &root).unwrap();
        assert_eq!(manifest.id, "hello-world");
        let widget = manifest.widget("counter").unwrap();
        assert_eq!((widget.width, widget.height), (4, 3));
        assert_eq!((widget.min_width, widget.min_height), (3, 2));
        assert_eq!(widget.refresh_interval_secs, None);
        assert_eq!(widget.fields.len(), 0);
    }

    #[test]
    fn rejects_empty_or_absolute_entries() {
        for bad in [
            "../outside.html",
            "/etc/passwd.html",
            "sub/../../up.html",
            "index.htm",
            "",
            "C:\\\\index.html",
        ] {
            let root = root_for("index.html");
            let text = format!(
                r#"
id = "p"
name = "P"
[[widgets]]
id = "w"
title = "W"
entry = "{bad}"
"#
            );
            let error = PluginManifest::parse(&text, &root).unwrap_err();
            assert!(
                error.to_string().contains("entry"),
                "expected entry rejection for '{bad}': {error}"
            );
        }
    }

    #[test]
    fn rejects_entry_missing_from_repository() {
        let root = write_plugin(&[("other.html", "<html></html>")]);
        let text = r#"
id = "p"
name = "P"
[[widgets]]
id = "w"
title = "W"
entry = "index.html"
"#;
        let error = PluginManifest::parse(text, &root).unwrap_err();
        assert!(error.to_string().contains("does not exist"), "{error}");
    }

    #[test]
    fn rejects_unknown_field_kinds_and_malformed_selects() {
        let root = root_for("index.html");
        let text = r#"
id = "p"
name = "P"
[[widgets]]
id = "w"
title = "W"
entry = "index.html"

[[widgets.fields]]
key = "theme"
label = "Theme"
kind = "shell"
"#;
        let error = PluginManifest::parse(text, &root).unwrap_err();
        assert!(error.to_string().contains("unknown kind"), "{error}");

        let text = text.replace("kind = \"shell\"", "kind = \"select\"");
        let error = PluginManifest::parse(&text, &root).unwrap_err();
        assert!(error.to_string().contains("at least one option"), "{error}");
    }

    #[test]
    fn rejects_bad_ids_and_duplicate_widgets() {
        let root = root_for("index.html");
        let text = r#"
id = "Bad Id"
name = "P"
[[widgets]]
id = "w"
title = "W"
entry = "index.html"
"#;
        let error = PluginManifest::parse(text, &root).unwrap_err();
        assert!(error.to_string().contains("plugin id"), "{error}");

        let text = r#"
id = "p"
name = "P"
[[widgets]]
id = "w"
title = "W"
entry = "index.html"
[[widgets]]
id = "w"
title = "W2"
entry = "index.html"
"#;
        let error = PluginManifest::parse(text, &root).unwrap_err();
        assert!(error.to_string().contains("twice"), "{error}");
    }

    #[test]
    fn size_bounds_are_enforced() {
        let root = root_for("index.html");
        let text = r#"
id = "p"
name = "P"
[[widgets]]
id = "w"
title = "W"
entry = "index.html"
width = 1
"#;
        let error = PluginManifest::parse(text, &root).unwrap_err();
        assert!(error.to_string().contains("between 2 and 12"), "{error}");

        let text = r#"
id = "p"
name = "P"
[[widgets]]
id = "w"
title = "W"
entry = "index.html"
width = 5
min_width = 6
"#;
        let error = PluginManifest::parse(text, &root).unwrap_err();
        assert!(
            error.to_string().contains("min_width must be between"),
            "{error}"
        );
    }

    #[test]
    fn slug_rules_match_spreadsheet_case() {
        assert!(is_valid_slug("hello-world"));
        assert!(is_valid_slug("a1_b"));
        assert!(is_valid_slug("a"));
        assert!(!is_valid_slug(""));
        assert!(!is_valid_slug("-leading"));
        assert!(!is_valid_slug("Has Space"));
        assert!(!is_valid_slug("under score"));
        assert!(!is_valid_slug(&"a".repeat(65)));
    }

    #[test]
    fn source_metadata_round_trips() {
        let source = PluginSource {
            source_url: "https://example.test/repo.git".to_string(),
            commit: "abc123".to_string(),
            installed_at: "2026-11-15T12:00:00Z".to_string(),
        };
        let text = source.to_toml().unwrap();
        assert_eq!(PluginSource::from_toml(&text).unwrap(), source);
    }
}
