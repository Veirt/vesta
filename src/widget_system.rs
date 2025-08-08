use async_trait::async_trait;
use axum::response::IntoResponse;
use maud::Markup;
use serde::Deserialize;
use std::{collections::HashMap, sync::Arc};

use crate::{
    AppState,
    config::{Service, Widget},
    error::{VestaError, VestaResult},
};

/// Query parameters for widget requests
#[derive(Deserialize, Debug, Clone)]
pub struct WidgetQuery {
    pub group: String,
    pub title: String,
}

/// Trait that all widgets must implement
#[async_trait]
pub trait WidgetHandler: Send + Sync {
    /// The name of the widget (should match the widget name in config)
    fn name(&self) -> &'static str;

    /// Render the widget as HTML
    fn render(&self, group_id: &str, service: &Service) -> Markup;

    /// Handle API requests for this widget
    async fn handle_request(&self, state: Arc<AppState>, query: WidgetQuery)
    -> VestaResult<Markup>;

    /// Validate widget configuration
    fn validate_config(&self, _widget: &Widget) -> VestaResult<()> {
        // Default implementation - no validation
        Ok(())
    }
}

/// Widget registry that manages all available widgets
pub struct WidgetRegistry {
    widgets: HashMap<String, Box<dyn WidgetHandler>>,
}

impl WidgetRegistry {
    pub fn new() -> Self {
        Self {
            widgets: HashMap::new(),
        }
    }

    /// Register a new widget
    pub fn register<W: WidgetHandler + 'static>(mut self, widget: W) -> Self {
        self.widgets
            .insert(widget.name().to_string(), Box::new(widget));
        self
    }

    /// Get a widget by name
    pub fn get(&self, name: &str) -> Option<&dyn WidgetHandler> {
        self.widgets.get(name).map(|w| w.as_ref())
    }

    /// Get all registered widget names
    pub fn get_registered_widgets(&self) -> Vec<&str> {
        self.widgets.keys().map(|s| s.as_str()).collect()
    }

    /// Validate all widgets in a configuration
    pub fn validate_widgets(&self, config: &crate::config::Dashboard) -> VestaResult<()> {
        for group in config.groups.values() {
            for service in &group.services {
                if let Some(widget_config) = &service.widget {
                    if let Some(widget) = self.get(&widget_config.name) {
                        widget.validate_config(widget_config).map_err(|e| {
                            VestaError::Internal(format!(
                                "Widget '{}' validation failed for service '{}': {}",
                                widget_config.name, service.title, e
                            ))
                        })?;
                    } else {
                        return Err(VestaError::Internal(format!(
                            "Unknown widget '{}' in service '{}'",
                            widget_config.name, service.title
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    /// Render a widget
    pub fn render_widget(&self, group_id: &str, service: &Service, widget: &Widget) -> Markup {
        if let Some(handler) = self.get(&widget.name) {
            handler.render(group_id, service)
        } else {
            maud::html! {
                div class="p-4 text-red-400 bg-red-900/20 border border-red-800 rounded-xl" {
                    "Unknown widget: " (widget.name)
                }
            }
        }
    }

    /// Handle a widget request
    pub async fn handle_widget_request(
        &self,
        widget_name: &str,
        state: Arc<AppState>,
        query: WidgetQuery,
    ) -> Result<impl IntoResponse + use<>, VestaError> {
        if let Some(handler) = self.get(widget_name) {
            let markup = handler.handle_request(state, query).await?;
            Ok(markup)
        } else {
            println!(
                "widgets: {} {:?}",
                widget_name,
                self.get_registered_widgets()
            );

            Err(VestaError::Internal(format!(
                "Unknown widget: {}",
                widget_name
            )))
        }
    }
}

impl Default for WidgetRegistry {
    fn default() -> Self {
        Self::new()
    }
}
