use async_trait::async_trait;
use maud::{html, Markup};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    config::{Service, Widget},
    error::{VestaError, VestaResult},
    widget_system::{WidgetHandler, WidgetQuery},
    AppState,
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
        let width = service.width.unwrap_or(1);
        let height = service.height.unwrap_or(1);

        html! {
            div class=(format!("bg-slate-900 border border-slate-800 rounded-xl p-4 h-full overflow-y-auto"))
                 style=(format!("grid-column: span {} / span {}; grid-row: span {} / span {};", width, width, height, height))
                 hx-get=(format!("/api/widgets/QuickLinks?group={}&title={}", group_id, service.title))
                 hx-trigger="load"
                 hx-swap="innerHTML" {
                div class="flex items-center justify-center h-full" {
                    div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500" {}
                }
            }
        }
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
            div class="space-y-3" {
                // Header
                div class="text-center" {
                    h3 class="text-lg font-semibold text-white mb-4" { "Quick Links" }
                }

                // Links grid
                div class="space-y-2" {
                    @for link in &links {
                        a href=(link.url) target="_blank"
                          class="flex items-center p-3 bg-slate-800 hover:bg-slate-700 rounded-lg transition-colors duration-200 group" {
                            @if let Some(icon) = &link.icon {
                                img src=(icon) alt=(link.title) class="w-6 h-6 mr-3 flex-shrink-0" {}
                            } @else {
                                div class="w-6 h-6 mr-3 flex-shrink-0 bg-blue-500 rounded flex items-center justify-center" {
                                    span class="text-white text-sm font-bold" {
                                        (link.title.chars().next().unwrap_or('?').to_uppercase())
                                    }
                                }
                            }
                            div class="flex-1" {
                                span class="text-white text-sm font-medium group-hover:text-blue-300 transition-colors" {
                                    (link.title)
                                }
                            }
                            div class="text-gray-400 group-hover:text-gray-300 transition-colors" {
                                svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" {
                                    path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                         d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" {}
                                }
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
