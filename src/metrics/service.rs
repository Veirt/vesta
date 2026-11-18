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

/// Owns every node poller. Each configured node runs one supervised task with
/// a fixed interval; shutting down aborts them all (every task is owned).
pub struct NodeService {
    stats: Arc<RwLock<HashMap<String, NodeStats>>>,
    scrape_failures: Arc<RwLock<HashSet<String>>>,
    updates: broadcast::Sender<(String, Option<NodeStats>)>,
    configuration: watch::Sender<Vec<NodeConfig>>,
    supervisor: JoinHandle<()>,
}

impl NodeService {
    /// Spawns one poller per node. Call `shutdown` to cancel them.
    pub fn start(nodes: &[NodeConfig], client: reqwest::Client) -> Arc<Self> {
        let exporter = Arc::new(NodeExporterCollector::new(client));
        let local = LocalCollector::new();
        let stats: Arc<RwLock<HashMap<String, NodeStats>>> = Arc::new(RwLock::new(HashMap::new()));
        let scrape_failures = Arc::new(RwLock::new(HashSet::new()));
        let (updates, _) = broadcast::channel(64);

        let (configuration, receiver) = watch::channel(nodes.to_vec());
        let supervisor = tokio::spawn(run_supervisor(
            receiver,
            exporter,
            local,
            Arc::clone(&stats),
            Arc::clone(&scrape_failures),
            updates.clone(),
        ));

        Arc::new(Self {
            stats,
            scrape_failures,
            updates,
            configuration,
            supervisor,
        })
    }

    pub async fn stats_for(&self, id: &str) -> Result<NodeStats, VestaError> {
        self.stats
            .read()
            .await
            .get(id)
            .cloned()
            .ok_or_else(|| VestaError::InvalidWidgetConfig(format!("unknown node '{id}'")))
    }

    pub async fn node_ids(&self) -> Vec<String> {
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

    pub async fn scrape_failed(&self, id: &str) -> bool {
        self.scrape_failures.read().await.contains(id)
    }

    pub fn subscribe_updates(&self) -> broadcast::Receiver<(String, Option<NodeStats>)> {
        self.updates.subscribe()
    }

    pub fn reconfigure(&self, nodes: &[NodeConfig]) -> Result<(), MetricsError> {
        crate::validation::validate_nodes(nodes).map_err(MetricsError)?;
        self.configuration.send_replace(nodes.to_vec());
        Ok(())
    }

    /// Aborts every poller; called on shutdown so no task outlives the service.
    pub fn shutdown(&self) {
        self.supervisor.abort();
    }
}

async fn run_supervisor(
    mut configuration: watch::Receiver<Vec<NodeConfig>>,
    exporter: Arc<NodeExporterCollector>,
    local: Arc<LocalCollector>,
    stats: Arc<RwLock<HashMap<String, NodeStats>>>,
    scrape_failures: Arc<RwLock<HashSet<String>>>,
    updates: broadcast::Sender<(String, Option<NodeStats>)>,
) {
    let mut pollers = JoinSet::new();
    loop {
        let nodes = configuration.borrow_and_update().clone();
        pollers.abort_all();
        while pollers.join_next().await.is_some() {}

        let configured_ids: HashSet<_> = nodes.iter().map(|node| node.id.as_str()).collect();
        stats
            .write()
            .await
            .retain(|id, _| configured_ids.contains(id.as_str()));
        scrape_failures
            .write()
            .await
            .retain(|id| configured_ids.contains(id.as_str()));

        for node in nodes {
            let exporter = Arc::clone(&exporter);
            let local = Arc::clone(&local);
            let stats = Arc::clone(&stats);
            let scrape_failures = Arc::clone(&scrape_failures);
            let updates = updates.clone();
            pollers.spawn(async move {
                poll_loop(node, exporter, local, stats, scrape_failures, updates).await;
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
    stats: Arc<RwLock<HashMap<String, NodeStats>>>,
    scrape_failures: Arc<RwLock<HashSet<String>>>,
    updates: broadcast::Sender<(String, Option<NodeStats>)>,
) {
    let mut ticker = tokio::time::interval(node.interval());
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let local = &local;
    let exporter = &exporter;
    let stats = &stats;
    let scrape_failures = &scrape_failures;
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
                stats
                    .write()
                    .await
                    .insert(node.id.clone(), new_stats.clone());
                scrape_failures.write().await.remove(&node.id);
                let _ = updates.send((node.id.clone(), Some(new_stats)));
            }
            Err(e) => {
                tracing::warn!(node = %node.id, error = %e, "node poll failed");
                scrape_failures.write().await.insert(node.id.clone());
                stats.write().await.remove(&node.id);
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
        assert!(crate::validation::validate_nodes(&[node("a"), node("b")]).is_ok());
        let err = crate::validation::validate_nodes(&[node("a"), node("a")]).unwrap_err();
        assert!(err.contains("duplicate node id 'a'"));
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
        assert!(service.scrape_failed("offline").await);
        service.shutdown();
    }
}
