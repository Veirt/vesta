use std::sync::Arc;

use tokio::sync::{Mutex, mpsc, oneshot};
use tokio::task::JoinHandle;

use crate::Settings;
use crate::config;
use crate::config_manager::ConfigManager;
use crate::database::Database;
use crate::error::{VestaError, VestaResult};
use crate::http_client::create_default_client;
use crate::metrics::service::NodeService;
use crate::plugins;
use crate::services::ping_service::PingService;
use crate::widget_system::{SharedWidgetRegistry, WidgetContext, WidgetHandlerDyn, WidgetRegistry};
use crate::widgets::adguard_home_widget::AdGuardHomeWidget;
use crate::widgets::clock_widget::ClockWidget;
use crate::widgets::jellyfin_random_show_widget::JellyfinRandomShowWidget;
use crate::widgets::prowlarr_widget::ProwlarrWidget;
use crate::widgets::quick_links_widget::QuickLinksWidget;
use crate::widgets::sonarr_calendar_widget::SonarrCalendarWidget;
use crate::widgets::system_stats_widget::SystemStatsWidget;
use crate::widgets::weather_widget::WeatherWidget;

const PLUGIN_LIFECYCLE_QUEUE_CAPACITY: usize = 8;

pub struct AppState {
    pub(crate) config: Arc<ConfigManager>,
    pub(crate) database: Arc<Database>,
    pub(crate) nodes: Arc<NodeService>,
    pub(crate) ping: PingService,
    pub(crate) widget_registry: SharedWidgetRegistry,
    pub(crate) plugins: Arc<plugins::manager::PluginManager>,
    pub(crate) plugin_ops: Arc<Mutex<()>>,
    plugin_requests: mpsc::Sender<PluginRequest>,
    _plugin_lifecycle_worker: JoinHandle<()>,
    #[cfg(test)]
    pub(crate) lifecycle_test_hooks: Arc<LifecycleTestHooks>,
    pub(crate) widget_ctx: WidgetContext,
    pub(crate) cookie_secure: bool,
}

enum PluginRequest {
    Install {
        url: String,
        reply: oneshot::Sender<VestaResult<plugins::manifest::PluginRecord>>,
    },
    Update {
        plugin_id: String,
        reply: oneshot::Sender<VestaResult<plugins::manifest::PluginRecord>>,
    },
    Uninstall {
        plugin_id: String,
        reply: oneshot::Sender<VestaResult<()>>,
    },
}

#[cfg(test)]
struct LifecycleTestPause {
    entered: oneshot::Sender<()>,
    release: oneshot::Receiver<()>,
}

#[cfg(test)]
#[derive(Default)]
pub(crate) struct LifecycleTestHooks {
    after_plugin_mutation: Mutex<Option<LifecycleTestPause>>,
    after_uninstall_check: Mutex<Option<LifecycleTestPause>>,
    dashboard_save_attempted: Mutex<Option<oneshot::Sender<()>>>,
}

#[cfg(test)]
impl LifecycleTestHooks {
    async fn pause_point(
        slot: &Mutex<Option<LifecycleTestPause>>,
        entered: oneshot::Sender<()>,
        release: oneshot::Receiver<()>,
    ) {
        *slot.lock().await = Some(LifecycleTestPause { entered, release });
    }

    async fn wait_at(slot: &Mutex<Option<LifecycleTestPause>>) {
        if let Some(pause) = slot.lock().await.take() {
            let _ = pause.entered.send(());
            let _ = pause.release.await;
        }
    }

    pub(crate) async fn hold_after_plugin_mutation(
        &self,
    ) -> (oneshot::Receiver<()>, oneshot::Sender<()>) {
        let (entered_tx, entered_rx) = oneshot::channel();
        let (release_tx, release_rx) = oneshot::channel();
        Self::pause_point(&self.after_plugin_mutation, entered_tx, release_rx).await;
        (entered_rx, release_tx)
    }

    pub(crate) async fn hold_after_uninstall_check(
        &self,
    ) -> (oneshot::Receiver<()>, oneshot::Sender<()>) {
        let (entered_tx, entered_rx) = oneshot::channel();
        let (release_tx, release_rx) = oneshot::channel();
        Self::pause_point(&self.after_uninstall_check, entered_tx, release_rx).await;
        (entered_rx, release_tx)
    }

    pub(crate) async fn watch_dashboard_save_attempt(&self) -> oneshot::Receiver<()> {
        let (attempted_tx, attempted_rx) = oneshot::channel();
        *self.dashboard_save_attempted.lock().await = Some(attempted_tx);
        attempted_rx
    }

    async fn plugin_mutation_finished(&self) {
        Self::wait_at(&self.after_plugin_mutation).await;
    }

    async fn uninstall_check_finished(&self) {
        Self::wait_at(&self.after_uninstall_check).await;
    }

    pub(crate) async fn dashboard_save_started(&self) {
        if let Some(attempted) = self.dashboard_save_attempted.lock().await.take() {
            let _ = attempted.send(());
        }
    }
}

async fn rebuild_widget_registry(
    builtin_widgets: &[Arc<dyn WidgetHandlerDyn>],
    plugins: &Arc<plugins::manager::PluginManager>,
    widget_registry: &SharedWidgetRegistry,
) -> VestaResult<()> {
    let builtins = builtin_widgets.to_vec();
    let plugins = Arc::clone(plugins);
    let next = tokio::task::spawn_blocking(move || plugins::build_registry(&builtins, &plugins))
        .await
        .map_err(|error| {
            VestaError::Plugin(format!("widget registry rebuild task failed: {error}"))
        })?;
    widget_registry.replace(next);
    Ok(())
}

async fn plugin_lifecycle_worker(
    mut requests: mpsc::Receiver<PluginRequest>,
    plugin_ops: Arc<Mutex<()>>,
    config: Arc<ConfigManager>,
    plugins: Arc<plugins::manager::PluginManager>,
    builtin_widgets: Vec<Arc<dyn WidgetHandlerDyn>>,
    widget_registry: SharedWidgetRegistry,
    #[cfg(test)] test_hooks: Arc<LifecycleTestHooks>,
) {
    while let Some(request) = requests.recv().await {
        let _lifecycle = Arc::clone(&plugin_ops).lock_owned().await;
        match request {
            PluginRequest::Install { url, reply } => {
                let result = match plugins.install(&url).await {
                    Ok(record) => {
                        #[cfg(test)]
                        test_hooks.plugin_mutation_finished().await;
                        rebuild_widget_registry(&builtin_widgets, &plugins, &widget_registry)
                            .await
                            .map(|()| record)
                    }
                    Err(error) => Err(error),
                };
                let _ = reply.send(result);
            }
            PluginRequest::Update { plugin_id, reply } => {
                let result = match plugins.update(&plugin_id).await {
                    Ok(record) => {
                        #[cfg(test)]
                        test_hooks.plugin_mutation_finished().await;
                        rebuild_widget_registry(&builtin_widgets, &plugins, &widget_registry)
                            .await
                            .map(|()| record)
                    }
                    Err(error) => Err(error),
                };
                let _ = reply.send(result);
            }
            PluginRequest::Uninstall { plugin_id, reply } => {
                let prefix = format!("{}{}{}", plugins::PLUGIN_NAME_PREFIX, plugin_id, ':');
                let in_use = config
                    .snapshot()
                    .items
                    .iter()
                    .any(|item| matches!(item, config::DashboardItem::Widget(widget) if widget.name.starts_with(&prefix)));
                let result = if in_use {
                    Err(VestaError::Plugin(format!(
                        "plugin '{plugin_id}' is still used by dashboard tiles; remove those tiles first"
                    )))
                } else {
                    #[cfg(test)]
                    test_hooks.uninstall_check_finished().await;
                    match plugins.uninstall(&plugin_id).await {
                        Ok(()) => {
                            #[cfg(test)]
                            test_hooks.plugin_mutation_finished().await;
                            rebuild_widget_registry(&builtin_widgets, &plugins, &widget_registry)
                                .await
                        }
                        Err(error) => Err(error),
                    }
                };
                let _ = reply.send(result);
            }
        }
    }
}

impl AppState {
    pub(crate) fn builtin_widgets(http_client: reqwest::Client) -> Vec<Arc<dyn WidgetHandlerDyn>> {
        vec![
            WidgetRegistry::handler(AdGuardHomeWidget::new(http_client.clone())),
            WidgetRegistry::handler(SonarrCalendarWidget::new(http_client.clone())),
            WidgetRegistry::handler(ProwlarrWidget::new(http_client.clone())),
            WidgetRegistry::handler(JellyfinRandomShowWidget::new(http_client.clone())),
            WidgetRegistry::handler(SystemStatsWidget::new()),
            WidgetRegistry::handler(WeatherWidget::new(http_client)),
            WidgetRegistry::handler(ClockWidget::new()),
            WidgetRegistry::handler(QuickLinksWidget::new()),
        ]
    }

    async fn send_plugin_request<T>(
        &self,
        build: impl FnOnce(oneshot::Sender<VestaResult<T>>) -> PluginRequest,
    ) -> VestaResult<T> {
        let (reply, result) = oneshot::channel();
        self.plugin_requests
            .send(build(reply))
            .await
            .map_err(|_| VestaError::Plugin("plugin lifecycle worker stopped".to_string()))?;
        result
            .await
            .map_err(|_| VestaError::Plugin("plugin lifecycle worker stopped".to_string()))?
    }

    pub(crate) async fn install_plugin(
        &self,
        url: &str,
    ) -> VestaResult<plugins::manifest::PluginRecord> {
        self.send_plugin_request(|reply| PluginRequest::Install {
            url: url.to_string(),
            reply,
        })
        .await
    }

    pub(crate) async fn update_plugin(
        &self,
        plugin_id: &str,
    ) -> VestaResult<plugins::manifest::PluginRecord> {
        self.send_plugin_request(|reply| PluginRequest::Update {
            plugin_id: plugin_id.to_string(),
            reply,
        })
        .await
    }

    pub(crate) async fn uninstall_plugin(&self, plugin_id: &str) -> VestaResult<()> {
        self.send_plugin_request(|reply| PluginRequest::Uninstall {
            plugin_id: plugin_id.to_string(),
            reply,
        })
        .await
    }

    pub(crate) async fn new(settings: &Settings) -> VestaResult<Arc<Self>> {
        let http_client = create_default_client()?;
        let database = Arc::new(Database::open(&settings.database_path).await?);
        let plugins = Arc::new(plugins::manager::PluginManager::new(&settings.plugins_dir)?);
        let builtin_widgets = Self::builtin_widgets(http_client.clone());
        let widget_registry =
            SharedWidgetRegistry::new(plugins::build_registry(&builtin_widgets, &plugins));

        let config = Arc::new(ConfigManager::new(
            &settings.config_path,
            widget_registry.clone(),
        )?);
        let plugin_ops = Arc::new(Mutex::new(()));
        let (plugin_requests, plugin_receiver) = mpsc::channel(PLUGIN_LIFECYCLE_QUEUE_CAPACITY);
        #[cfg(test)]
        let lifecycle_test_hooks = Arc::new(LifecycleTestHooks::default());
        let plugin_lifecycle_worker = tokio::spawn(plugin_lifecycle_worker(
            plugin_receiver,
            Arc::clone(&plugin_ops),
            Arc::clone(&config),
            Arc::clone(&plugins),
            builtin_widgets.clone(),
            widget_registry.clone(),
            #[cfg(test)]
            Arc::clone(&lifecycle_test_hooks),
        ));
        let nodes = NodeService::start(&config.snapshot().nodes, http_client.clone());
        let ping = PingService::new(http_client);
        let widget_ctx = WidgetContext {
            nodes: nodes.clone(),
        };

        Ok(Arc::new(Self {
            config,
            database,
            nodes,
            ping,
            widget_registry,
            plugins,
            plugin_ops,
            plugin_requests,
            _plugin_lifecycle_worker: plugin_lifecycle_worker,
            #[cfg(test)]
            lifecycle_test_hooks,
            widget_ctx,
            cookie_secure: settings.cookie_secure,
        }))
    }
}

#[cfg(test)]
pub(crate) async fn test_app_state(
    prefix: &str,
) -> (plugins::test_fixtures::TempDir, Settings, Arc<AppState>) {
    let sandbox = plugins::test_fixtures::temp_dir(prefix);
    let static_dir = sandbox.path().join("static");
    std::fs::create_dir_all(&static_dir).unwrap();
    let settings = Settings {
        config_path: sandbox.path().join("config.toml").display().to_string(),
        database_path: sandbox.path().join("vesta.db").display().to_string(),
        plugins_dir: sandbox.path().join("plugins").display().to_string(),
        bind_addr: "127.0.0.1:0".to_string(),
        static_dir: static_dir.display().to_string(),
        cookie_secure: false,
    };
    let state = AppState::new(&settings).await.unwrap();
    (sandbox, settings, state)
}
