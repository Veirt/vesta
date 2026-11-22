use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use serde::{Deserialize, Deserializer, Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::config::Widget;
use crate::error::{VestaError, VestaResult};

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

pub fn refresh_interval_field(default: u64) -> WidgetField {
    WidgetField::builtin(
        "refresh_interval",
        "Refresh every (seconds)",
        "number",
        true,
        "How often this widget requests updated values (1–3600 seconds).",
    )
    .with_default(serde_json::json!(default))
    .with_range(1.0, 3600.0)
}

fn number_from<'de, T: DeserializeOwned, D>(
    deserializer: D,
    parse: fn(&str) -> Option<T>,
) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match &value {
        Value::Number(_) => T::deserialize(value).map_err(serde::de::Error::custom),
        Value::String(s) => {
            parse(s).ok_or_else(|| serde::de::Error::custom(format!("'{s}' is not a valid number")))
        }
        other => Err(serde::de::Error::custom(format!(
            "expected a number, got {other}"
        ))),
    }
}

pub fn debounced_u64<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    number_from(deserializer, |s| s.parse().ok())
}

pub fn debounced_f64<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
    number_from(deserializer, |s| s.parse().ok())
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

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct DynamicOption {
    pub value: String,
    pub label: String,
}

#[derive(Clone)]
pub struct WidgetContext {
    pub nodes: Arc<crate::metrics::service::NodeService>,
}

#[async_trait]
pub trait WidgetHandler: Send + Sync {
    fn name(&self) -> &str;

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

    fn catalog_visible(&self) -> bool {
        true
    }

    fn fields(&self) -> Vec<WidgetField> {
        Vec::new()
    }

    fn default_size(&self) -> (u8, u8) {
        (4, 2)
    }

    fn min_size(&self) -> (u8, u8) {
        (3, 2)
    }

    type Config: DeserializeOwned + Send;

    fn parse_config(&self, raw: Option<&crate::config::WidgetConfig>) -> VestaResult<Self::Config> {
        parse_widget_options(raw)
    }

    fn refresh_interval_secs(&self, _config: &Self::Config) -> Option<u64> {
        None
    }

    async fn handle_request(
        &self,
        ctx: WidgetContext,
        query: WidgetQuery,
        config: Self::Config,
    ) -> VestaResult<Value>;

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

pub fn parse_widget_options<T: DeserializeOwned>(
    raw: Option<&crate::config::WidgetConfig>,
) -> VestaResult<T> {
    static EMPTY: std::sync::OnceLock<crate::config::WidgetConfig> = std::sync::OnceLock::new();
    let options = raw.unwrap_or_else(|| EMPTY.get_or_init(Default::default));
    let value = serde_json::to_value(options)
        .map_err(|e| VestaError::InvalidWidgetConfig(format!("widget options: {e}")))?;
    serde_json::from_value(value).map_err(|e| VestaError::InvalidWidgetConfig(format!("{e}")))
}

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
        #[serde(deserialize_with = "debounced_u64")]
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

    #[derive(serde::Deserialize)]
    struct DebouncedSample {
        #[serde(deserialize_with = "debounced_u64")]
        count: u64,
        #[serde(deserialize_with = "debounced_f64")]
        ratio: f64,
    }

    #[test]
    fn debounced_numbers_accept_native_numbers() {
        let sample: DebouncedSample = serde_json::from_value(serde_json::json!({
            "count": 5, "ratio": 0.5
        }))
        .unwrap();
        assert_eq!(sample.count, 5);
        assert_eq!(sample.ratio, 0.5);
    }

    #[test]
    fn debounced_numbers_accept_strings_that_parse() {
        let sample: DebouncedSample = serde_json::from_value(serde_json::json!({
            "count": "5", "ratio": "0.5"
        }))
        .unwrap();
        assert_eq!(sample.count, 5);
        assert_eq!(sample.ratio, 0.5);
    }

    #[test]
    fn debounced_numbers_reject_non_numeric_strings() {
        let result = serde_json::from_value::<DebouncedSample>(serde_json::json!({
            "count": "five", "ratio": 1.0
        }));
        assert!(result.is_err());
    }
}
