use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::config::WidgetConfig;
use crate::error::VestaResult;
use crate::widget_system::{
    WidgetContext, WidgetField, WidgetHandler, WidgetQuery, parse_widget_options,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Link {
    pub title: String,
    pub url: String,
    #[serde(default)]
    pub icon: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// One entry per link, declared as `widget.config.links = [{...}, ...]`.
    #[serde(default)]
    pub links: Vec<Link>,
}

impl Config {
    fn from_raw(raw: Option<&WidgetConfig>) -> VestaResult<Self> {
        let mut config: Self = parse_widget_options(raw)?;
        for link in &mut config.links {
            if link
                .icon
                .as_deref()
                .is_some_and(|icon| icon.trim().is_empty())
            {
                link.icon = None;
            }
            if link.title.trim().is_empty() || !crate::validation::valid_link(&link.url) {
                return Err(crate::error::VestaError::InvalidWidgetConfig(
                    "each quick link needs a title and a relative or http(s) destination"
                        .to_string(),
                ));
            }
            if link.icon.as_deref().is_some_and(|icon| {
                !icon.trim().is_empty() && !crate::validation::valid_image_url(icon)
            }) {
                return Err(crate::error::VestaError::InvalidWidgetConfig(format!(
                    "icon for quick link '{}' must be an http(s) URL or uploaded icon path",
                    link.title
                )));
            }
        }
        Ok(config)
    }
}

pub struct QuickLinksWidget;

impl QuickLinksWidget {
    pub fn new() -> Self {
        Self
    }
}

impl Default for QuickLinksWidget {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WidgetHandler for QuickLinksWidget {
    fn name(&self) -> &str {
        "QuickLinks"
    }

    fn description(&self) -> &str {
        "Shows a compact list of links with optional remote or uploaded icons."
    }

    fn default_size(&self) -> (u8, u8) {
        (3, 2)
    }

    fn min_size(&self) -> (u8, u8) {
        (3, 2)
    }

    fn fields(&self) -> Vec<WidgetField> {
        vec![
            WidgetField::builtin(
                "links",
                "Links",
                "links",
                false,
                "Each link can have a title, destination, and optional icon URL or upload.",
            )
            .with_default(serde_json::json!([])),
        ]
    }

    type Config = Config;

    fn parse_config(&self, raw: Option<&WidgetConfig>) -> VestaResult<Config> {
        Config::from_raw(raw)
    }

    async fn handle_request(
        &self,
        _ctx: WidgetContext,
        _query: WidgetQuery,
        config: Config,
    ) -> VestaResult<Value> {
        Ok(json!({ "links": config.links }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_links_array() {
        let options = json!({
            "links": [
                {"title": "GitHub", "url": "https://github.com"},
                {"title": "Docs", "url": "https://docs.rs", "icon": "https://x/icon.svg"}
            ]
        });
        let options: WidgetConfig = serde_json::from_value(options).unwrap();
        let config = Config::from_raw(Some(&options)).unwrap();
        assert_eq!(config.links.len(), 2);
        assert_eq!(config.links[0].title, "GitHub");
        assert!(config.links[1].icon.is_some());
    }
}
