use async_trait::async_trait;
use maud::{Markup, html};
use serde::Deserialize;

use crate::config::{Service, WidgetConfig};
use crate::error::VestaResult;
use crate::widget_system::{
    WidgetContext, WidgetHandler, WidgetRef, parse_widget_options, widget_scaffold,
};

#[derive(Clone, Debug, Deserialize)]
pub struct Link {
    pub title: String,
    pub url: String,
    #[serde(default)]
    pub icon: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    /// One entry per link, declared as `widget.config.links = [{...}, ...]`.
    #[serde(default)]
    pub links: Vec<Link>,
}

impl Config {
    /// Also accepts the legacy dotted style (`link_0_title`, `link_0_url`, …).
    fn from_raw(raw: Option<&WidgetConfig>) -> VestaResult<Self> {
        let parsed: Self = parse_widget_options(raw)?;
        if !parsed.links.is_empty() {
            return Ok(parsed);
        }
        let Some(options) = raw else {
            return Ok(parsed);
        };
        let links = legacy_links(options);
        Ok(Self { links })
    }
}

/// Reads `link_<i>_title` / `link_<i>_url` / `link_<i>_icon` until an index
/// is missing, so old configs keep working unchanged.
fn legacy_links(options: &WidgetConfig) -> Vec<Link> {
    let get = |key: String| {
        options
            .get(&key)
            .and_then(|v| v.as_str().map(str::to_string))
    };
    let mut links = Vec::new();
    let mut index = 0;
    while let Some(title) = get(format!("link_{index}_title")) {
        let Some(url) = get(format!("link_{index}_url")) else {
            break;
        };
        let icon = get(format!("link_{index}_icon"));
        links.push(Link { title, url, icon });
        index += 1;
    }
    links
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

fn link_row(link: &Link) -> Markup {
    html! {
        a href=(link.url) target="_blank" rel="noreferrer"
          class="flex items-center p-2.5 bg-zinc-800/60 hover:bg-zinc-700/60 border border-zinc-700/50 hover:border-violet-500/30 rounded-md transition-all duration-150 group cursor-pointer" {
            @if let Some(icon) = &link.icon {
                img src=(icon) alt=(link.title) class="w-5 h-5 mr-2.5 flex-shrink-0 opacity-80" {}
            } @else {
                div class="w-5 h-5 mr-2.5 flex-shrink-0 bg-violet-500/20 border border-violet-500/30 rounded flex items-center justify-center" {
                    span class="text-violet-400 text-xs font-bold font-mono" {
                        (link.title.chars().next().unwrap_or('?').to_uppercase())
                    }
                }
            }
            span class="text-zinc-300 text-sm font-medium group-hover:text-zinc-100 transition-colors flex-1 truncate" {
                (link.title)
            }
            svg class="w-3.5 h-3.5 text-zinc-600 group-hover:text-zinc-400 transition-colors flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24" {
                path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                     d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" {}
            }
        }
    }
}

#[async_trait]
impl WidgetHandler for QuickLinksWidget {
    fn name(&self) -> &'static str {
        "QuickLinks"
    }

    type Config = Config;

    fn parse_config(&self, raw: Option<&WidgetConfig>) -> VestaResult<Config> {
        Config::from_raw(raw)
    }

    fn render(&self, mount: &WidgetRef, service: &Service, _config: &Config) -> Markup {
        widget_scaffold(service, mount, self.name(), None)
    }

    async fn handle_request(
        &self,
        _ctx: WidgetContext,
        _mount: WidgetRef,
        config: Config,
    ) -> VestaResult<Markup> {
        Ok(html! {
            div class="space-y-2" {
                h3 class="text-xs font-semibold text-zinc-400 uppercase tracking-widest mb-3"
                    style="font-family: 'JetBrains Mono', monospace;" {
                    "Quick Links"
                }
                div class="space-y-1" {
                    @for link in &config.links {
                        (link_row(link))
                    }
                }
            }
        })
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

    #[test]
    fn parses_legacy_dotted_links() {
        let options: WidgetConfig = serde_json::from_value(json!({
            "link_0_title": "GitHub",
            "link_0_url": "https://github.com",
            "link_1_title": "Blog",
            "link_1_url": "https://blog.example",
            "link_1_icon": "https://blog.example/favicon.ico"
        }))
        .unwrap();
        let config = Config::from_raw(Some(&options)).unwrap();
        assert_eq!(config.links.len(), 2);
        assert_eq!(config.links[1].title, "Blog");
        assert_eq!(
            config.links[1].icon.as_deref(),
            Some("https://blog.example/favicon.ico")
        );
    }
}
