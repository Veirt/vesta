use std::collections::HashMap;
use std::sync::Arc;

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
    pub description: &'static str,
    pub default_width: u8,
    pub default_height: u8,
    pub min_width: u8,
    pub min_height: u8,
    pub fields: Vec<WidgetField>,
}

#[derive(Clone, Debug, Serialize)]
pub struct WidgetField {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: &'static str,
    pub required: bool,
    pub description: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<WidgetOption>,
}

#[derive(Clone, Debug, Serialize)]
pub struct WidgetOption {
    pub value: &'static str,
    pub label: &'static str,
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
    pub config: Arc<crate::config_manager::ConfigManager>,
}

/// What a widget provides. `Config` is the widget's typed options schema,
/// deserialized from its JSON settings map; validation happens by construction
/// (bad config = typed error at load, not surprises at request time).
#[async_trait]
pub trait WidgetHandler: Send + Sync {
    /// Registry key, matching `widget.name` in the config (e.g. "SystemStats").
    fn name(&self) -> &'static str;

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

    fn description(&self) -> &'static str {
        "Configure this built-in widget."
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

/// Registry mapping widget names to handlers. Built once at startup; all
/// later lookups (render, validate, request handling) go through it.
pub struct WidgetRegistry {
    widgets: HashMap<String, Box<dyn WidgetHandlerDyn>>,
}

impl WidgetRegistry {
    pub fn new() -> Self {
        Self {
            widgets: HashMap::new(),
        }
    }

    pub fn register<W: WidgetHandler + 'static>(mut self, widget: W) -> Self {
        self.widgets
            .insert(widget.name().to_string(), Box::new(DynWidget::new(widget)));
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

    pub fn descriptors(&self) -> Vec<WidgetDescriptor> {
        self.names()
            .into_iter()
            .filter_map(|name| self.get(name))
            .map(|handler| handler.descriptor())
            .collect()
    }

    pub fn refresh_interval_secs(&self, widget: &Widget) -> Option<u64> {
        self.get(&widget.name)?
            .refresh_interval_secs(widget)
            .ok()
            .flatten()
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
        ctx: WidgetContext,
        query: WidgetQuery,
    ) -> VestaResult<WidgetResponseData> {
        let handler = self
            .get(widget_name)
            .ok_or_else(|| VestaError::WidgetNotFound {
                id: query.id.clone(),
            })?;
        handler.handle_erased(ctx, query).await
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
    fn descriptor(&self) -> WidgetDescriptor;
    fn validate_erased(&self, widget: &Widget) -> VestaResult<()>;
    fn refresh_interval_secs(&self, widget: &Widget) -> VestaResult<Option<u64>>;
    fn min_size_erased(&self) -> (u8, u8);
    async fn options_erased(
        &self,
        field: &str,
        config_json: &Value,
    ) -> VestaResult<Vec<DynamicOption>>;
    async fn handle_erased(
        &self,
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
    fn descriptor(&self) -> WidgetDescriptor {
        let (default_width, default_height) = self.inner.default_size();
        let (min_width, min_height) = self.inner.min_size();
        WidgetDescriptor {
            name: self.inner.name().to_string(),
            title: self.inner.title(),
            description: self.inner.description(),
            default_width,
            default_height,
            min_width,
            min_height,
            fields: self.inner.fields(),
        }
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
        ctx: WidgetContext,
        query: WidgetQuery,
    ) -> VestaResult<WidgetResponseData> {
        let widget = ctx
            .config
            .widget(&query.id)?
            .ok_or_else(|| VestaError::WidgetNotFound {
                id: query.id.clone(),
            })?;
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
        fn name(&self) -> &'static str {
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
