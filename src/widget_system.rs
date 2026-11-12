use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use maud::{Markup, html};
use serde::de::DeserializeOwned;

use crate::config::{Service, Widget};
use crate::error::{VestaError, VestaResult};

/// Identifies one widget instance on the dashboard: its group and service
/// title in the config, which double as its API address. HTML attribute
/// escaping makes the composed query string safe for any title.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WidgetRef {
    pub group: String,
    pub title: String,
}

impl WidgetRef {
    pub fn new(group: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            group: group.into(),
            title: title.into(),
        }
    }

    /// The URL htmx fetches the widget's data fragment from.
    pub fn api_path(&self, widget: &str) -> String {
        format!(
            "/api/widgets/{widget}?group={}&title={}",
            self.group, self.title
        )
    }
}

/// Query parameters for widget data requests.
#[derive(serde::Deserialize, Debug, Clone)]
pub struct WidgetQuery {
    pub group: String,
    pub title: String,
}

impl From<WidgetQuery> for WidgetRef {
    fn from(q: WidgetQuery) -> Self {
        WidgetRef::new(q.group, q.title)
    }
}

/// Services a widget may use, passed explicitly instead of global state.
#[derive(Clone)]
pub struct WidgetContext {
    pub nodes: Arc<crate::metrics::service::NodeService>,
    pub config: Arc<crate::config_manager::ConfigManager>,
}

/// What a widget provides. `Config` is the widget's typed options schema,
/// deserialized from the TOML string map; validation happens by construction
/// (bad config = typed error at load, not surprises at request time).
#[async_trait]
pub trait WidgetHandler: Send + Sync {
    /// Registry key, matching `widget.name` in the config (e.g. "SystemStats").
    fn name(&self) -> &'static str;

    /// The widget's typed configuration.
    type Config: DeserializeOwned + Send;

    /// Parse and validate the widget's options. Errors name the offending key.
    fn parse_config(&self, raw: Option<&crate::config::WidgetConfig>) -> VestaResult<Self::Config> {
        parse_widget_options(raw)
    }

    /// Server-side HTML for the widget's shell (the htmx mount point).
    fn render(&self, mount: &WidgetRef, service: &Service, config: &Self::Config) -> Markup;

    /// Fresh data fragment for htmx to swap into the shell.
    async fn handle_request(
        &self,
        ctx: WidgetContext,
        mount: WidgetRef,
        config: Self::Config,
    ) -> VestaResult<Markup>;
}

/// Typed config plumbing: widget config structs deserialize from the raw
/// options map. TOML values may be strings or native numbers; the
/// `widget_config` helpers coerce strings to numbers where needed.
pub fn parse_widget_options<T: DeserializeOwned>(
    raw: Option<&crate::config::WidgetConfig>,
) -> VestaResult<T> {
    static EMPTY: std::sync::OnceLock<crate::config::WidgetConfig> = std::sync::OnceLock::new();
    let options = raw.unwrap_or_else(|| EMPTY.get_or_init(Default::default));
    let value = serde_json::to_value(options)
        .map_err(|e| VestaError::InvalidWidgetConfig(format!("widget options: {e}")))?;
    serde_json::from_value(value).map_err(|e| {
        VestaError::InvalidWidgetConfig(format!(
            "{e} (all TOML values are strings; numeric fields accept any parseable string)"
        ))
    })
}

/// The shared shell every widget renders into: grid sizing plus an htmx
/// loader that fetches the data fragment on load, and on an interval when set.
pub fn widget_scaffold(
    service: &Service,
    mount: &WidgetRef,
    widget: &str,
    refresh_secs: Option<u64>,
) -> Markup {
    widget_scaffold_with(service, mount, widget, refresh_secs, loading_placeholder())
}

pub fn widget_scaffold_with(
    service: &Service,
    mount: &WidgetRef,
    widget: &str,
    refresh_secs: Option<u64>,
    initial: Markup,
) -> Markup {
    let trigger = match refresh_secs {
        Some(secs) => format!("load, every {secs}s"),
        None => "load".to_string(),
    };
    html! {
        div class="bg-zinc-900 border border-zinc-800 rounded-lg p-4 h-full overflow-y-auto"
            style=(grid_span_style(service.width, service.height)) {
            div class="h-full"
                hx-get=(mount.api_path(widget))
                hx-trigger=(trigger)
                hx-swap="innerHTML" {
                (initial)
            }
        }
    }
}

pub fn loading_placeholder() -> Markup {
    html! {
        div class="flex items-center justify-center h-full" {
            div class="animate-spin rounded-full h-6 w-6 border-b-2 border-violet-500" {}
        }
    }
}

/// Small labelled stat row shared by data-heavy widgets.
pub fn stat_row(label: &str, value: &str, bar: Option<Markup>) -> Markup {
    html! {
        div class="space-y-1.5" {
            div class="flex justify-between items-center" {
                span class="text-xs text-zinc-400 uppercase tracking-wide" { (label) }
                span class="text-xs text-zinc-200 font-mono" { (value) }
            }
            @if let Some(bar) = bar { (bar) }
        }
    }
}

pub fn usage_bar(percent: f64) -> Markup {
    let color = match percent {
        p if p < 50.0 => "bg-emerald-500",
        p if p < 80.0 => "bg-amber-400",
        _ => "bg-red-500",
    };
    let percentage = percent.clamp(0.0, 100.0);
    html! {
        div class="w-full bg-zinc-800 rounded-full h-1.5" {
            div class=(format!("h-1.5 rounded-full transition-all duration-300 {color}"))
                style=(format!("width: {percentage}%")) {}
        }
    }
}

fn grid_span_style(width: Option<u8>, height: Option<u8>) -> String {
    let w = width.unwrap_or(1).max(1);
    let h = height.unwrap_or(1).max(1);
    format!("grid-column: span {w}; grid-row: span {h};")
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

    /// Render the shell for a service's widget, or an inline error card.
    pub fn render_widget(&self, mount: &WidgetRef, service: &Service, widget: &Widget) -> Markup {
        match self.get(&widget.name) {
            Some(handler) => match handler.render_erased(mount, service, widget) {
                Ok(markup) => markup,
                Err(e) => error_card(&format!("widget '{}': {e}", widget.name)),
            },
            None => error_card(&format!("unknown widget '{}'", widget.name)),
        }
    }

    /// Validate every widget config in the dashboard against its handler.
    pub fn validate_dashboard(&self, config: &crate::config::Dashboard) -> VestaResult<()> {
        for (group_id, _, service) in config.services() {
            if let Some(widget) = &service.widget {
                let handler = self.get(&widget.name).ok_or_else(|| {
                    VestaError::InvalidWidgetConfig(format!(
                        "unknown widget '{}' on service '{}' (group '{group_id}'); available: {}",
                        widget.name,
                        service.title,
                        self.names().join(", ")
                    ))
                })?;
                handler.validate_erased(widget).map_err(|e| {
                    VestaError::InvalidWidgetConfig(format!(
                        "widget '{}' on service '{}': {e}",
                        widget.name, service.title
                    ))
                })?;
            }
        }
        Ok(())
    }

    pub async fn handle_request(
        &self,
        widget_name: &str,
        ctx: WidgetContext,
        query: WidgetQuery,
    ) -> VestaResult<Markup> {
        let handler = self
            .get(widget_name)
            .ok_or_else(|| VestaError::WidgetNotFound {
                group: widget_name.to_string(),
                title: query.title.clone(),
            })?;
        handler.handle_erased(ctx, query).await
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
    fn render_erased(
        &self,
        mount: &WidgetRef,
        service: &Service,
        widget: &Widget,
    ) -> VestaResult<Markup>;
    fn validate_erased(&self, widget: &Widget) -> VestaResult<()>;
    async fn handle_erased(&self, ctx: WidgetContext, query: WidgetQuery) -> VestaResult<Markup>;
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
    fn render_erased(
        &self,
        mount: &WidgetRef,
        service: &Service,
        widget: &Widget,
    ) -> VestaResult<Markup> {
        let config = self.inner.parse_config(widget.config.as_ref())?;
        Ok(self.inner.render(mount, service, &config))
    }

    fn validate_erased(&self, widget: &Widget) -> VestaResult<()> {
        self.inner.parse_config(widget.config.as_ref()).map(|_| ())
    }

    async fn handle_erased(&self, ctx: WidgetContext, query: WidgetQuery) -> VestaResult<Markup> {
        let mount = WidgetRef::from(query.clone());
        let widget = ctx
            .config
            .widget(&query.group, &query.title)?
            .ok_or_else(|| VestaError::WidgetNotFound {
                group: query.group.clone(),
                title: query.title.clone(),
            })?;
        let config = self.inner.parse_config(widget.config.as_ref())?;
        self.inner.handle_request(ctx, mount, config).await
    }
}

fn error_card(message: &str) -> Markup {
    html! {
        div class="p-4 text-red-400 bg-red-900/20 border border-red-800 rounded-xl" {
            (message)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::WidgetConfig;
    use maud::html;

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

        fn render(&self, mount: &WidgetRef, _service: &Service, config: &FakeConfig) -> Markup {
            html! { div data-node=(config.node) data-refresh=(config.refresh_interval) hx-get=(mount.api_path(self.name())) {} }
        }

        async fn handle_request(
            &self,
            _ctx: WidgetContext,
            _mount: WidgetRef,
            config: FakeConfig,
        ) -> VestaResult<Markup> {
            Ok(html! { span { (config.node) } })
        }
    }

    fn service_with(widget: Widget) -> Service {
        Service {
            title: "svc".to_string(),
            href: None,
            img_src: None,
            width: None,
            height: None,
            widget: Some(widget),
            ping: None,
        }
    }

    fn options(value: serde_json::Value) -> WidgetConfig {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn scaffold_embeds_htmx_attributes_and_grid_span() {
        let service = Service {
            title: "svc".to_string(),
            href: None,
            img_src: None,
            width: Some(2),
            height: Some(3),
            widget: None,
            ping: None,
        };
        // maud escapes `&` in attributes; browsers decode it back.
        let markup = widget_scaffold(&service, &WidgetRef::new("g", "t"), "Fake", Some(30))
            .0
            .to_string();
        assert!(
            markup.contains("hx-get=\"/api/widgets/Fake?group=g&amp;title=t\""),
            "{markup}"
        );
        assert!(markup.contains("hx-trigger=\"load, every 30s\""));
        assert!(markup.contains("grid-column: span 2"));
        assert!(markup.contains("grid-row: span 3"));
    }

    #[test]
    fn registry_validates_unknown_widget_names() {
        let registry = WidgetRegistry::new().register(FakeWidget);
        let mut groups = indexmap::IndexMap::new();
        groups.insert(
            "g".to_string(),
            crate::config::Group {
                name: "G".to_string(),
                columns: 1,
                services: vec![service_with(Widget {
                    name: "Nope".to_string(),
                    config: None,
                })],
            },
        );
        let dashboard = crate::config::Dashboard {
            nodes: vec![],
            groups,
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
            name: "Fake".to_string(),
            config: Some(options(serde_json::json!({"refresh_interval": "soon"}))),
        };
        let service = service_with(widget);
        let err = registry
            .render_widget(
                &WidgetRef::new("g", "svc"),
                &service,
                service.widget.as_ref().unwrap(),
            )
            .0;
        assert!(err.contains("widget 'Fake'"), "{err}");
    }

    #[test]
    fn typed_config_coerces_string_numbers() {
        let widget = Widget {
            name: "Fake".to_string(),
            config: Some(options(serde_json::json!({
                "node": "nas",
                "refresh_interval": "42"
            }))),
        };
        let registry = WidgetRegistry::new().register(FakeWidget);
        let markup = registry
            .render_widget(
                &WidgetRef::new("g", "t"),
                &service_with(widget),
                &Widget {
                    name: "Fake".to_string(),
                    config: Some(options(serde_json::json!({
                        "node": "nas", "refresh_interval": "42"
                    }))),
                },
            )
            .0;
        assert!(markup.contains("data-node=\"nas\""), "{markup}");
        assert!(markup.contains("data-refresh=\"42\""), "{markup}");
    }
}
