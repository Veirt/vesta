use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::config::Widget;
use crate::error::{VestaError, VestaResult};

/// Query parameters for widget data requests.
#[derive(serde::Deserialize, Debug, Clone)]
pub struct WidgetQuery {
    pub id: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct WidgetResponseData {
    pub widget: Value,
    pub refresh_interval_secs: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct WidgetDescriptor {
    pub name: String,
    pub title: String,
    pub description: Cow<'static, str>,
    pub default_width: u8,
    pub default_height: u8,
    pub min_width: u8,
    pub min_height: u8,
    pub fields: Vec<WidgetField>,
}

#[derive(Clone, Debug, Serialize)]
pub struct WidgetField {
    pub key: Cow<'static, str>,
    pub label: Cow<'static, str>,
    pub kind: Cow<'static, str>,
    pub required: bool,
    pub description: Cow<'static, str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<WidgetOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
}

impl WidgetField {
    pub fn builtin(
        key: &'static str,
        label: &'static str,
        kind: &'static str,
        required: bool,
        description: &'static str,
    ) -> Self {
        Self {
            key: Cow::Borrowed(key),
            label: Cow::Borrowed(label),
            kind: Cow::Borrowed(kind),
            required,
            description: Cow::Borrowed(description),
            default: None,
            options: Vec::new(),
            min: None,
            max: None,
        }
    }

    pub fn with_default(mut self, default: serde_json::Value) -> Self {
        self.default = Some(default);
        self
    }

    pub fn with_options(mut self, options: Vec<WidgetOption>) -> Self {
        self.options = options;
        self
    }

    pub fn with_range(mut self, min: f64, max: f64) -> Self {
        self.min = Some(min);
        self.max = Some(max);
        self
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct WidgetOption {
    pub value: Cow<'static, str>,
    pub label: Cow<'static, str>,
}

impl WidgetOption {
    pub fn builtin(value: &'static str, label: &'static str) -> Self {
        Self {
            value: Cow::Borrowed(value),
            label: Cow::Borrowed(label),
        }
    }
}

/// A select option resolved at edit time from an external system, for
/// fields whose choices depend on the widget's own configuration (the
/// values are not known at startup).
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct DynamicOption {
    pub value: String,
    pub label: String,
}

/// Services a widget may use, passed explicitly instead of global state.
#[derive(Clone)]
pub struct WidgetContext {
    pub nodes: Arc<crate::metrics::service::NodeService>,
}

/// What a widget provides. `Config` is the widget's typed options schema,
/// deserialized from its JSON settings map; validation happens by construction
/// (bad config = typed error at load, not surprises at request time).
#[async_trait]
pub trait WidgetHandler: Send + Sync {
    /// Registry key, matching `widget.name` in the config (e.g. "SystemStats").
    fn name(&self) -> &str;

    /// Human-readable label for menus and catalogs; defaults to the key split
    /// at inner uppercase letters ("SonarrCalendar" -> "Sonarr Calendar").
    fn title(&self) -> String {
        let name = self.name();
        let mut title = String::with_capacity(name.len() + 4);
        for (index, character) in name.char_indices() {
            if index > 0
                && character.is_uppercase()
                && title.chars().last().is_some_and(|previous| previous != ' ')
            {
                title.push(' ');
            }
            title.push(character);
        }
        title
    }

    fn description(&self) -> &str {
        "Configure this built-in widget."
    }

    /// Whether this widget should appear in the Settings new-tile catalog.
    /// Legacy-only widgets override this to return `false` while continuing
    /// to render and validate existing tiles.
    fn catalog_visible(&self) -> bool {
        true
    }

    fn fields(&self) -> Vec<WidgetField> {
        Vec::new()
    }

    /// Default dashboard dimensions in grid columns and rows.
    fn default_size(&self) -> (u8, u8) {
        (4, 2)
    }

    /// Smallest usable dimensions in grid columns and rows.
    fn min_size(&self) -> (u8, u8) {
        (3, 2)
    }

    /// The widget's typed configuration.
    type Config: DeserializeOwned + Send;

    /// Parse and validate the widget's options. Errors name the offending key.
    fn parse_config(&self, raw: Option<&crate::config::WidgetConfig>) -> VestaResult<Self::Config> {
        parse_widget_options(raw)
    }

    /// Optional refresh cadence exposed to the client as safe metadata.
    fn refresh_interval_secs(&self, _config: &Self::Config) -> Option<u64> {
        None
    }

    /// Widget data serialized as JSON for the React renderer.
    async fn handle_request(
        &self,
        ctx: WidgetContext,
        query: WidgetQuery,
        config: Self::Config,
    ) -> VestaResult<Value>;

    /// Options for a dynamic select field, resolved from the widget's
    /// unsaved form values (`config_json` is the raw options map as typed
    /// in the editor). Widgets with only static selects need no override.
    async fn field_options(
        &self,
        field: &str,
        _config_json: &Value,
    ) -> VestaResult<Vec<DynamicOption>> {
        Err(VestaError::InvalidWidgetConfig(format!(
            "field '{field}' has no dynamic options"
        )))
    }
}

/// Typed config plumbing: widget config structs deserialize from the raw
/// JSON options map.
pub fn parse_widget_options<T: DeserializeOwned>(
    raw: Option<&crate::config::WidgetConfig>,
) -> VestaResult<T> {
    static EMPTY: std::sync::OnceLock<crate::config::WidgetConfig> = std::sync::OnceLock::new();
    let options = raw.unwrap_or_else(|| EMPTY.get_or_init(Default::default));
    let value = serde_json::to_value(options)
        .map_err(|e| VestaError::InvalidWidgetConfig(format!("widget options: {e}")))?;
    serde_json::from_value(value).map_err(|e| VestaError::InvalidWidgetConfig(format!("{e}")))
}

/// The widget registry is rebuilt when plugins are installed or removed, so
/// both request handling (`AppState`) and dashboard validation
/// (`ConfigManager`) hold a shared handle and always observe the current
/// snapshot.
#[derive(Clone)]
pub struct SharedWidgetRegistry {
    inner: Arc<RwLock<Arc<WidgetRegistry>>>,
}

impl SharedWidgetRegistry {
    pub fn new(registry: WidgetRegistry) -> Self {
        Self {
            inner: Arc::new(RwLock::new(Arc::new(registry))),
        }
    }

    pub fn current(&self) -> Arc<WidgetRegistry> {
        self.inner
            .read()
            .expect("widget registry lock poisoned")
            .clone()
    }

    pub fn replace(&self, registry: WidgetRegistry) {
        *self.inner.write().expect("widget registry lock poisoned") = Arc::new(registry);
    }
}

/// Registry mapping widget names to handlers. Rebuilt atomically on startup and
/// plugin mutation; all lookups (render, validate, request handling) go
/// through it.
pub struct WidgetRegistry {
    widgets: HashMap<String, Arc<dyn WidgetHandlerDyn>>,
}

impl WidgetRegistry {
    pub fn new() -> Self {
        Self {
            widgets: HashMap::new(),
        }
    }

    pub fn handler<W: WidgetHandler + 'static>(widget: W) -> Arc<dyn WidgetHandlerDyn> {
        Arc::new(DynWidget::new(widget))
    }

    pub fn register<W: WidgetHandler + 'static>(self, widget: W) -> Self {
        self.register_shared(Self::handler(widget))
    }

    pub fn register_shared(mut self, handler: Arc<dyn WidgetHandlerDyn>) -> Self {
        self.widgets.insert(handler.name().to_string(), handler);
        self
    }

    pub fn get(&self, name: &str) -> Option<&dyn WidgetHandlerDyn> {
        self.widgets.get(name).map(|w| w.as_ref())
    }

    pub fn names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.widgets.keys().map(|s| s.as_str()).collect();
        names.sort_unstable();
        names
    }

    pub fn catalog_descriptors(&self) -> Vec<WidgetDescriptor> {
        self.names()
            .into_iter()
            .filter_map(|name| self.get(name))
            .filter(|handler| handler.catalog_visible())
            .map(|handler| handler.descriptor())
            .collect()
    }

    pub fn refresh_interval_secs(&self, widget: &Widget) -> Option<u64> {
        self.get(&widget.name)?
            .refresh_interval_secs(widget)
            .ok()
            .flatten()
    }

    pub fn default_size(&self, name: &str) -> Option<(u8, u8)> {
        self.get(name).map(|handler| handler.default_size_erased())
    }

    pub fn min_size(&self, name: &str) -> Option<(u8, u8)> {
        self.get(name).map(|handler| handler.min_size_erased())
    }

    /// Validate every widget config in the dashboard against its handler.
    pub fn validate_dashboard(&self, config: &crate::config::Dashboard) -> VestaResult<()> {
        for widget in config.widgets() {
            let handler = self.get(&widget.name).ok_or_else(|| {
                VestaError::InvalidWidgetConfig(format!(
                    "unknown widget '{}' on tile '{}'; available: {}",
                    widget.name,
                    widget.id,
                    self.names().join(", ")
                ))
            })?;
            handler.validate_erased(widget).map_err(|error| {
                VestaError::InvalidWidgetConfig(format!(
                    "widget '{}' on tile '{}': {error}",
                    widget.name, widget.id
                ))
            })?;
        }
        Ok(())
    }

    pub async fn handle_request(
        &self,
        widget_name: &str,
        widget: &Widget,
        ctx: WidgetContext,
        query: WidgetQuery,
    ) -> VestaResult<WidgetResponseData> {
        let handler = self
            .get(widget_name)
            .ok_or_else(|| VestaError::WidgetNotFound {
                id: query.id.clone(),
            })?;
        handler.handle_erased(widget, ctx, query).await
    }

    /// Resolve the dynamic options of one widget field from unsaved form
    /// values (see `WidgetHandler::field_options`).
    pub async fn field_options(
        &self,
        widget_name: &str,
        field: &str,
        config_json: &Value,
    ) -> VestaResult<Vec<DynamicOption>> {
        let handler = self.get(widget_name).ok_or_else(|| {
            VestaError::InvalidWidgetConfig(format!(
                "unknown widget '{widget_name}'; available: {}",
                self.names().join(", ")
            ))
        })?;
        handler.options_erased(field, config_json).await
    }
}

impl Default for WidgetRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Type-erased view of a handler so the registry can store widgets with
/// different `Config` associated types behind one map.
#[async_trait]
pub trait WidgetHandlerDyn: Send + Sync {
    fn name(&self) -> &str;
    fn catalog_visible(&self) -> bool;
    fn descriptor(&self) -> WidgetDescriptor;
    fn validate_erased(&self, widget: &Widget) -> VestaResult<()>;
    fn refresh_interval_secs(&self, widget: &Widget) -> VestaResult<Option<u64>>;
    fn default_size_erased(&self) -> (u8, u8);
    fn min_size_erased(&self) -> (u8, u8);
    async fn options_erased(
        &self,
        field: &str,
        config_json: &Value,
    ) -> VestaResult<Vec<DynamicOption>>;
    async fn handle_erased(
        &self,
        widget: &Widget,
        ctx: WidgetContext,
        query: WidgetQuery,
    ) -> VestaResult<WidgetResponseData>;
}

struct DynWidget<W: WidgetHandler + 'static> {
    inner: W,
}

impl<W: WidgetHandler + 'static> DynWidget<W> {
    fn new(widget: W) -> Self {
        Self { inner: widget }
    }
}

#[async_trait]
impl<W: WidgetHandler + 'static> WidgetHandlerDyn for DynWidget<W> {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn catalog_visible(&self) -> bool {
        self.inner.catalog_visible()
    }

    fn descriptor(&self) -> WidgetDescriptor {
        let (default_width, default_height) = self.inner.default_size();
        let (min_width, min_height) = self.inner.min_size();
        WidgetDescriptor {
            name: self.inner.name().to_string(),
            title: self.inner.title(),
            description: Cow::Owned(self.inner.description().to_string()),
            default_width,
            default_height,
            min_width,
            min_height,
            fields: self.inner.fields(),
        }
    }

    fn default_size_erased(&self) -> (u8, u8) {
        self.inner.default_size()
    }

    fn min_size_erased(&self) -> (u8, u8) {
        self.inner.min_size()
    }

    async fn options_erased(
        &self,
        field: &str,
        config_json: &Value,
    ) -> VestaResult<Vec<DynamicOption>> {
        self.inner.field_options(field, config_json).await
    }

    fn validate_erased(&self, widget: &Widget) -> VestaResult<()> {
        self.inner.parse_config(widget.config.as_ref()).map(|_| ())
    }

    fn refresh_interval_secs(&self, widget: &Widget) -> VestaResult<Option<u64>> {
        let config = self.inner.parse_config(widget.config.as_ref())?;
        Ok(self.inner.refresh_interval_secs(&config))
    }

    async fn handle_erased(
        &self,
        widget: &Widget,
        ctx: WidgetContext,
        query: WidgetQuery,
    ) -> VestaResult<WidgetResponseData> {
        if widget.name != self.inner.name() {
            return Err(VestaError::WidgetNotFound { id: query.id });
        }
        let config = self.inner.parse_config(widget.config.as_ref())?;
        let refresh_interval_secs = self.inner.refresh_interval_secs(&config);
        let widget = self.inner.handle_request(ctx, query, config).await?;
        Ok(WidgetResponseData {
            widget,
            refresh_interval_secs,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::WidgetConfig;

    struct FakeWidget;

    #[derive(Clone, Debug, serde::Deserialize)]
    struct FakeConfig {
        #[serde(default = "default_name")]
        node: String,
        #[serde(deserialize_with = "crate::widget_config::debounced_u64")]
        refresh_interval: u64,
    }

    fn default_name() -> String {
        "first".to_string()
    }

    #[async_trait]
    impl WidgetHandler for FakeWidget {
        fn name(&self) -> &str {
            "Fake"
        }

        type Config = FakeConfig;

        fn refresh_interval_secs(&self, config: &FakeConfig) -> Option<u64> {
            Some(config.refresh_interval)
        }

        async fn handle_request(
            &self,
            _ctx: WidgetContext,
            _query: WidgetQuery,
            config: FakeConfig,
        ) -> VestaResult<Value> {
            Ok(serde_json::json!({ "node": config.node }))
        }
    }

    fn options(value: serde_json::Value) -> WidgetConfig {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn registry_validates_unknown_widget_names() {
        let registry = WidgetRegistry::new().register(FakeWidget);
        let dashboard = crate::config::Dashboard {
            nodes: vec![],
            items: vec![crate::config::DashboardItem::Widget(Widget {
                id: "bad-widget".to_string(),
                title: "Bad widget".to_string(),
                name: "Nope".to_string(),
                group: None,
                config: None,
                x: None,
                y: None,
                width: None,
                height: None,
            })],
            groups: vec![],
        };
        let err = registry.validate_dashboard(&dashboard).unwrap_err();
        assert!(err.to_string().contains("unknown widget 'Nope'"), "{err}");
        assert!(err.to_string().contains("available: Fake"), "{err}");
    }

    #[test]
    fn registry_rejects_invalid_widget_config_at_load() {
        let registry = WidgetRegistry::new().register(FakeWidget);
        // refresh_interval as a non-numeric string must fail validation.
        let widget = Widget {
            id: "fake".to_string(),
            title: "Fake".to_string(),
            name: "Fake".to_string(),
            group: None,
            config: Some(options(serde_json::json!({"refresh_interval": "soon"}))),
            x: None,
            y: None,
            width: None,
            height: None,
        };
        let err = registry
            .get("Fake")
            .unwrap()
            .validate_erased(&widget)
            .unwrap_err();
        assert!(err.to_string().contains("soon"), "{err}");
    }

    #[test]
    fn typed_config_coerces_string_numbers() {
        let widget = Widget {
            id: "fake".to_string(),
            title: "Fake".to_string(),
            name: "Fake".to_string(),
            group: None,
            config: Some(options(serde_json::json!({
                "node": "nas",
                "refresh_interval": "42"
            }))),
            x: None,
            y: None,
            width: None,
            height: None,
        };
        let registry = WidgetRegistry::new().register(FakeWidget);
        registry
            .get("Fake")
            .unwrap()
            .validate_erased(&widget)
            .unwrap();
        assert_eq!(registry.refresh_interval_secs(&widget), Some(42));
    }
}
