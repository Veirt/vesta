use async_trait::async_trait;
use maud::{Markup, html};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    AppState,
    config::{Service, Widget},
    error::{VestaError, VestaResult},
    widget_system::{WidgetHandler, WidgetQuery},
    widgets::widget_container,
};

#[derive(Deserialize, Debug)]
pub struct QuickLink {
    pub title: String,
    pub url: String,
    pub icon: Option<String>,
}

pub struct QuickLinksWidget;

impl QuickLinksWidget {
    pub fn new() -> Self {
        Self
    }

    fn parse_config(
        &self,
        config: &std::collections::HashMap<String, String>,
    ) -> VestaResult<Vec<QuickLink>> {
        let mut links = Vec::new();
        let mut i = 0;

        // Parse links in format: link_0_title, link_0_url, link_0_icon, link_1_title, etc.
        while let Some(title) = config.get(&format!("link_{}_title", i)) {
            if let Some(url) = config.get(&format!("link_{}_url", i)) {
                let icon = config.get(&format!("link_{}_icon", i)).cloned();
                links.push(QuickLink {
                    title: title.clone(),
                    url: url.clone(),
                    icon,
                });
                i += 1;
            } else {
                break;
            }
        }

        if links.is_empty() {
            return Err(VestaError::Internal(
                "No links configured for QuickLinks widget".to_string(),
            ));
        }

        Ok(links)
    }
}

#[async_trait]
impl WidgetHandler for QuickLinksWidget {
    fn name(&self) -> &'static str {
        "QuickLinks"
    }

    fn render(&self, group_id: &str, service: &Service) -> Markup {
        widget_container(
            service.width,
            service.height,
            "overflow-y-auto",
            html! {
                div
                    class="h-full"
                    hx-get=(format!("/api/widgets/QuickLinks?group={}&title={}", group_id, service.title))
                    hx-trigger="load"
                    hx-swap="innerHTML" {
                    div class="flex items-center justify-center h-full" {
                        div class="animate-spin rounded-full h-6 w-6 border-b-2 border-violet-500" {}
                    }
                }
            },
        )
    }

    async fn handle_request(
        &self,
        state: Arc<AppState>,
        query: WidgetQuery,
    ) -> VestaResult<Markup> {
        let config_manager = &state.config_manager;
        let service = config_manager
            .get_service(&query.group, &query.title)?
            .ok_or_else(|| VestaError::Internal("Service not found".to_string()))?;

        let widget_config = service
            .widget
            .as_ref()
            .and_then(|w| w.config.as_ref())
            .ok_or_else(|| {
                VestaError::Internal("QuickLinks widget config not found".to_string())
            })?;

        let links = self.parse_config(widget_config)?;

        Ok(html! {
            div class="space-y-2" {
                // Header
                h3 class="text-xs font-semibold text-zinc-400 uppercase tracking-widest mb-3" style="font-family: 'JetBrains Mono', monospace;" { "Quick Links" }

                // Links
                div class="space-y-1" {
                    @for link in &links {
                        a href=(link.url) target="_blank"
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
            }
        })
    }

    fn validate_config(&self, widget: &Widget) -> VestaResult<()> {
        let config = widget
            .config
            .as_ref()
            .ok_or_else(|| VestaError::Internal("QuickLinks widget requires config".to_string()))?;

        // Check if at least one link is configured
        if !config.contains_key("link_0_title") || !config.contains_key("link_0_url") {
            return Err(VestaError::Internal(
                "QuickLinks widget requires at least one link (link_0_title and link_0_url)"
                    .to_string(),
            ));
        }

        // Validate URLs
        let mut i = 0;
        while let Some(url) = config.get(&format!("link_{}_url", i)) {
            if !url.starts_with("http://") && !url.starts_with("https://") {
                return Err(VestaError::Internal(format!(
                    "Invalid URL for link_{}: URLs must start with http:// or https://",
                    i
                )));
            }
            i += 1;
        }

        Ok(())
    }
}
