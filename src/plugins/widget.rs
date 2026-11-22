use std::borrow::Cow;

use async_trait::async_trait;

use crate::config::WidgetConfig;
use crate::error::{VestaError, VestaResult};
use crate::plugins::manifest::{PluginFieldSpec, PluginOptionSpec, PluginWidgetSpec};
use crate::widget_system::{WidgetContext, WidgetField, WidgetHandler, WidgetOption, WidgetQuery};
use serde_json::{Value, json};

pub const PLUGIN_NAME_PREFIX: &str = "Plugin:";

#[derive(Clone)]
pub struct PluginWidgetHandler {
    plugin_id: String,
    widget_name: String,
    spec: PluginWidgetSpec,
}

impl PluginWidgetHandler {
    pub fn new(plugin_id: &str, spec: PluginWidgetSpec) -> Self {
        let widget_name = format!("{PLUGIN_NAME_PREFIX}{plugin_id}:{}", spec.id);
        Self {
            plugin_id: plugin_id.to_string(),
            widget_name,
            spec,
        }
    }
}

fn to_widget_field(field: &PluginFieldSpec) -> WidgetField {
    WidgetField {
        key: Cow::Owned(field.key.clone()),
        label: Cow::Owned(field.label.clone()),
        kind: Cow::Owned(field.kind.clone()),
        required: field.required,
        description: Cow::Owned(field.description.as_deref().unwrap_or("").to_string()),
        default: field.default.clone(),
        options: field.options.iter().map(to_widget_option).collect(),
        min: None,
        max: None,
    }
}

fn to_widget_option(option: &PluginOptionSpec) -> WidgetOption {
    WidgetOption {
        value: Cow::Owned(option.value.clone()),
        label: Cow::Owned(option.label.clone()),
    }
}

#[async_trait]
impl WidgetHandler for PluginWidgetHandler {
    fn name(&self) -> &str {
        &self.widget_name
    }

    fn title(&self) -> String {
        self.spec.title.clone()
    }

    fn description(&self) -> &str {
        self.spec.description.as_deref().unwrap_or("Plugin widget.")
    }

    fn fields(&self) -> Vec<WidgetField> {
        self.spec.fields.iter().map(to_widget_field).collect()
    }

    fn default_size(&self) -> (u8, u8) {
        (self.spec.width, self.spec.height)
    }

    fn min_size(&self) -> (u8, u8) {
        (self.spec.min_width, self.spec.min_height)
    }

    type Config = WidgetConfig;

    fn parse_config(&self, raw: Option<&WidgetConfig>) -> VestaResult<Self::Config> {
        let config = raw.cloned().unwrap_or_default();
        for field in &self.spec.fields {
            if !field.required {
                continue;
            }
            let present = match config.get(&field.key) {
                Some(Value::String(text)) => !text.trim().is_empty(),
                Some(Value::Null) | None => false,
                Some(_) => true,
            };
            if !present {
                return Err(VestaError::InvalidWidgetConfig(format!(
                    "widget '{}': field '{}' is required",
                    self.spec.title, field.label
                )));
            }
        }
        Ok(config)
    }

    fn refresh_interval_secs(&self, _config: &Self::Config) -> Option<u64> {
        self.spec.refresh_interval_secs
    }

    async fn handle_request(
        &self,
        _ctx: WidgetContext,
        _query: WidgetQuery,
        config: Self::Config,
    ) -> VestaResult<Value> {
        Ok(json!({
            "plugin_id": self.plugin_id,
            "widget_id": self.spec.id,
            "entry": self.spec.entry,
            "config": config,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn spec(fields: Vec<PluginFieldSpec>) -> PluginWidgetSpec {
        PluginWidgetSpec {
            id: "counter".to_string(),
            title: "Counter".to_string(),
            description: Some("A sample counter.".to_string()),
            entry: "index.html".to_string(),
            width: 3,
            height: 2,
            min_width: 3,
            min_height: 2,
            refresh_interval_secs: Some(66),
            fields,
        }
    }

    fn config_of(entries: &[(&str, Value)]) -> WidgetConfig {
        entries
            .iter()
            .map(|(key, value)| (key.to_string(), value.clone()))
            .collect::<HashMap<String, Value>>()
    }

    fn handler() -> PluginWidgetHandler {
        PluginWidgetHandler::new("hello-world", spec(vec![]))
    }

    #[test]
    fn widget_name_uses_prefixed_three_part_key() {
        assert_eq!(handler().name(), "Plugin:hello-world:counter");
        assert!(!handler().name().starts_with("Plugin:{}"));
    }

    #[test]
    fn descriptor_comes_from_the_manifest_spec() {
        let handler = handler();
        assert_eq!(handler.description(), "A sample counter.");
        assert_eq!(handler.default_size(), (3, 2));
        assert_eq!(handler.min_size(), (3, 2));
        assert_eq!(handler.refresh_interval_secs(&config_of(&[])), Some(66));
        assert!(handler.fields().is_empty());
    }

    #[test]
    fn required_fields_must_be_present_and_non_empty() {
        let handler = PluginWidgetHandler::new(
            "hello-world",
            spec(vec![PluginFieldSpec {
                key: "label".to_string(),
                label: "Label".to_string(),
                kind: "text".to_string(),
                required: true,
                description: None,
                default: None,
                options: vec![],
            }]),
        );
        for bad in [
            config_of(&[]),
            config_of(&[("label", Value::Null)]),
            config_of(&[("label", json!("   "))]),
        ] {
            let error = handler.parse_config(Some(&bad)).unwrap_err();
            assert!(error.to_string().contains("required"), "{error}");
        }
        handler
            .parse_config(Some(&config_of(&[("label", json!(" ok "))])))
            .unwrap();
        handler
            .parse_config(Some(&config_of(&[("label", json!(42))])))
            .unwrap();
    }

    #[tokio::test]
    async fn data_response_points_at_the_asset_bundle() {
        let nodes = crate::metrics::service::NodeService::start(
            &[],
            crate::http_client::create_default_client().unwrap(),
        );
        let ctx = WidgetContext {
            nodes: nodes.clone(),
        };

        let response = PluginWidgetHandler::new("hello-world", spec(vec![]))
            .handle_request(
                ctx,
                WidgetQuery {
                    id: "item-1".to_string(),
                },
                config_of(&[("start_value", json!("0"))]),
            )
            .await
            .unwrap();
        nodes.shutdown();
        assert_eq!(response["plugin_id"], json!("hello-world"));
        assert_eq!(response["widget_id"], json!("counter"));
        assert_eq!(response["entry"], json!("index.html"));
        assert_eq!(response["config"]["start_value"], json!("0"));
    }
}
