use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tokio::sync::{RwLock, broadcast, watch};
use tokio::task::{JoinHandle, JoinSet};

use crate::error::{MetricsError, VestaError};
use crate::metrics::local::LocalCollector;
use crate::metrics::node::{NodeConfig, NodeStats};
use crate::metrics::node_exporter::NodeExporterCollector;

fn sysinfo_error(e: sysinfo::Error) -> MetricsError {
    MetricsError(format!("local stats: {e}"))
}

#[derive(Clone)]
pub enum NodeObservation {
    Stats(NodeStats),
    Failed,
}

/// Owns every node poller. Each configured node runs one supervised task with
/// a fixed interval; shutting down aborts them all (every task is owned).
pub struct NodeService {
    exporter: Arc<NodeExporterCollector>,
    observations: Arc<RwLock<HashMap<String, NodeObservation>>>,
    updates: broadcast::Sender<(String, Option<NodeStats>)>,
    configuration: watch::Sender<Vec<NodeConfig>>,
    supervisor: JoinHandle<()>,
}

impl NodeService {
    /// Spawns one poller per node. Call `shutdown` to cancel them.
    pub fn start(nodes: &[NodeConfig], client: reqwest::Client) -> Arc<Self> {
        let exporter = Arc::new(NodeExporterCollector::new(client));
        exporter.set_active_endpoints(&remote_exporter_endpoints(nodes));
        let local = LocalCollector::new();
        let observations: Arc<RwLock<HashMap<String, NodeObservation>>> =
            Arc::new(RwLock::new(HashMap::new()));
        let (updates, _) = broadcast::channel(64);

        let (configuration, receiver) = watch::channel(nodes.to_vec());
        let supervisor = tokio::spawn(run_supervisor(
            receiver,
            Arc::clone(&exporter),
            local,
            Arc::clone(&observations),
            updates.clone(),
        ));

        Arc::new(Self {
            exporter,
            observations,
            updates,
            configuration,
            supervisor,
        })
    }

    pub async fn observation(&self, id: &str) -> Option<NodeObservation> {
        self.observations.read().await.get(id).cloned()
    }

    pub async fn stats_for(&self, id: &str) -> Result<NodeStats, VestaError> {
        match self.observation(id).await {
            Some(NodeObservation::Stats(stats)) => Ok(stats),
            Some(NodeObservation::Failed) | None => Err(VestaError::InvalidWidgetConfig(format!(
                "unknown node '{id}'"
            ))),
        }
    }

    pub fn node_ids(&self) -> Vec<String> {
        self.configuration
            .borrow()
            .iter()
            .map(|node| node.id.clone())
            .collect()
    }

    pub fn node_name(&self, id: &str) -> Option<String> {
        self.configuration
            .borrow()
            .iter()
            .find(|node| node.id == id)
            .map(|node| node.name.clone())
    }

    pub fn subscribe_updates(&self) -> broadcast::Receiver<(String, Option<NodeStats>)> {
        self.updates.subscribe()
    }

    pub fn reconfigure(&self, nodes: &[NodeConfig]) -> Result<(), MetricsError> {
        crate::validation::nodes::validate_nodes(nodes).map_err(MetricsError)?;
        self.exporter
            .set_active_endpoints(&remote_exporter_endpoints(nodes));
        self.configuration.send_replace(nodes.to_vec());
        Ok(())
    }

    /// Aborts every poller; called on shutdown so no task outlives the service.
    pub fn shutdown(&self) {
        self.supervisor.abort();
    }
}

fn remote_exporter_endpoints(nodes: &[NodeConfig]) -> HashSet<String> {
    nodes
        .iter()
        .filter(|node| !node.is_local())
        .map(|node| node.endpoint.clone())
        .collect()
}

async fn run_supervisor(
    mut configuration: watch::Receiver<Vec<NodeConfig>>,
    exporter: Arc<NodeExporterCollector>,
    local: Arc<LocalCollector>,
    observations: Arc<RwLock<HashMap<String, NodeObservation>>>,
    updates: broadcast::Sender<(String, Option<NodeStats>)>,
) {
    let mut pollers = JoinSet::new();
    loop {
        let nodes = configuration.borrow_and_update().clone();
        pollers.abort_all();
        while pollers.join_next().await.is_some() {}

        let configured_ids: HashSet<_> = nodes.iter().map(|node| node.id.as_str()).collect();
        observations
            .write()
            .await
            .retain(|id, _| configured_ids.contains(id.as_str()));

        for node in nodes {
            let exporter = Arc::clone(&exporter);
            let local = Arc::clone(&local);
            let observations = Arc::clone(&observations);
            let updates = updates.clone();
            pollers.spawn(async move {
                poll_loop(node, exporter, local, observations, updates).await;
            });
        }

        tokio::select! {
            changed = configuration.changed() => {
                if changed.is_err() {
                    break;
                }
            }
            Some(result) = pollers.join_next(), if !pollers.is_empty() => {
                if let Err(error) = result {
                    tracing::error!(%error, "node poller exited unexpectedly");
                } else {
                    tracing::error!("node poller exited unexpectedly");
                }
            }
        }
    }
    pollers.abort_all();
}

async fn poll_loop(
    node: NodeConfig,
    exporter: Arc<NodeExporterCollector>,
    local: Arc<LocalCollector>,
    observations: Arc<RwLock<HashMap<String, NodeObservation>>>,
    updates: broadcast::Sender<(String, Option<NodeStats>)>,
) {
    let mut ticker = tokio::time::interval(node.interval());
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let local = &local;
    let exporter = &exporter;
    let observations = &observations;
    loop {
        ticker.tick().await;
        let result = if node.is_local() {
            let local = Arc::clone(local);
            tokio::task::spawn_blocking(move || local.collect())
                .await
                .map_err(|e| MetricsError(format!("local collect task: {e}")))
                .and_then(|r| r.map_err(sysinfo_error))
        } else {
            exporter.scrape(&node.endpoint).await
        };

        match result {
            Ok(new_stats) => {
                observations
                    .write()
                    .await
                    .insert(node.id.clone(), NodeObservation::Stats(new_stats.clone()));
                let _ = updates.send((node.id.clone(), Some(new_stats)));
            }
            Err(e) => {
                tracing::warn!(node = %node.id, error = %e, "node poll failed");
                observations
                    .write()
                    .await
                    .insert(node.id.clone(), NodeObservation::Failed);
                let _ = updates.send((node.id.clone(), None));
                continue;
            }
        };
    }
}

/// Validates that node ids are unique before persistence and poller updates.
#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str) -> NodeConfig {
        NodeConfig {
            id: id.to_string(),
            name: id.to_string(),
            hostname: None,
            device_name: None,
            icon: None,
            endpoint: "local".to_string(),
            interval_secs: 15,
        }
    }

    #[test]
    fn rejects_duplicate_node_ids() {
        assert!(crate::validation::nodes::validate_nodes(&[node("a"), node("b")]).is_ok());
        let err = crate::validation::nodes::validate_nodes(&[node("a"), node("a")]).unwrap_err();
        assert!(err.contains("duplicate node id 'a'"));
    }

    #[test]
    fn exporter_endpoints_include_only_configured_remote_nodes() {
        let local = node("local");
        let remote = NodeConfig {
            endpoint: "https://nas.example/metrics".to_string(),
            ..node("nas")
        };

        assert_eq!(
            remote_exporter_endpoints(&[local, remote]),
            HashSet::from(["https://nas.example/metrics".to_string()])
        );
    }

    #[tokio::test]
    async fn unknown_node_is_an_error() {
        let service = NodeService::start(&[], reqwest::Client::new());
        assert!(service.stats_for("nope").await.is_err());
        service.shutdown();
    }

    #[tokio::test]
    async fn local_node_reports_stats_after_first_poll() {
        let config = NodeConfig {
            interval_secs: 1,
            ..node("self")
        };
        let service = NodeService::start(&[config], reqwest::Client::new());
        let mut updates = service.subscribe_updates();
        let (id, stats) = tokio::time::timeout(std::time::Duration::from_secs(5), updates.recv())
            .await
            .expect("local node should publish within 5s")
            .expect("update channel should remain open");
        assert_eq!(id, "self");
        let stats = stats.expect("successful scrapes should include metrics");
        assert!(stats.memory_usage_percent() <= 100.0);
        assert!(service.stats_for("self").await.is_ok());
        service.shutdown();
    }

    #[tokio::test]
    async fn failed_scrapes_are_published_and_tracked() {
        let config = NodeConfig {
            id: "offline".to_string(),
            name: "Offline".to_string(),
            hostname: None,
            device_name: None,
            icon: None,
            endpoint: "http://127.0.0.1:1/metrics".to_string(),
            interval_secs: 1,
        };
        let service = NodeService::start(&[config], reqwest::Client::new());
        let mut updates = service.subscribe_updates();
        let (id, stats) = tokio::time::timeout(std::time::Duration::from_secs(5), updates.recv())
            .await
            .expect("failed scrape should publish within 5s")
            .expect("update channel should remain open");

        assert_eq!(id, "offline");
        assert!(stats.is_none());
        assert!(matches!(
            service.observation("offline").await,
            Some(NodeObservation::Failed)
        ));
        service.shutdown();
    }
}
