use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tokio::task::JoinHandle;

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
    pollers: Vec<JoinHandle<()>>,
}

impl NodeService {
    /// Spawns one poller per node. Call `shutdown` to cancel them.
    pub fn start(nodes: &[NodeConfig], client: reqwest::Client) -> Arc<Self> {
        let exporter = Arc::new(NodeExporterCollector::new(client));
        let local = LocalCollector::new();
        let stats: Arc<RwLock<HashMap<String, NodeStats>>> = Arc::new(RwLock::new(HashMap::new()));

        let pollers = nodes
            .iter()
            .map(|node| {
                let node = node.clone();
                let exporter = Arc::clone(&exporter);
                let local = Arc::clone(&local);
                let stats = Arc::clone(&stats);
                tokio::spawn(async move {
                    poll_loop(node, exporter, local, stats).await;
                })
            })
            .collect();

        Arc::new(Self { stats, pollers })
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
        self.stats.read().await.keys().cloned().collect()
    }

    /// Aborts every poller; called on shutdown so no task outlives the service.
    pub fn shutdown(&self) {
        for handle in &self.pollers {
            handle.abort();
        }
    }
}

async fn poll_loop(
    node: NodeConfig,
    exporter: Arc<NodeExporterCollector>,
    local: Arc<LocalCollector>,
    stats: Arc<RwLock<HashMap<String, NodeStats>>>,
) {
    let mut ticker = tokio::time::interval(node.interval());
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let local = &local;
    let exporter = &exporter;
    let stats = &stats;
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
            Ok(new_stats) => stats.write().await.insert(node.id.clone(), new_stats),
            Err(e) => {
                tracing::warn!(node = %node.id, error = %e, "node poll failed");
                continue;
            }
        };
    }
}

/// Validates that node ids are unique; config loading calls this once.
pub fn validate_nodes(nodes: &[NodeConfig]) -> Result<(), MetricsError> {
    let mut ids = std::collections::HashSet::new();
    for node in nodes {
        if !ids.insert(node.id.as_str()) {
            return Err(MetricsError(format!("duplicate node id '{}'", node.id)));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str) -> NodeConfig {
        NodeConfig {
            id: id.to_string(),
            name: id.to_string(),
            endpoint: "local".to_string(),
            interval_secs: 15,
        }
    }

    #[test]
    fn rejects_duplicate_node_ids() {
        assert!(validate_nodes(&[node("a"), node("b")]).is_ok());
        let err = validate_nodes(&[node("a"), node("a")]).unwrap_err();
        assert!(err.to_string().contains("duplicate node id 'a'"));
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
        // First tick lands immediately after spawn; allow scheduler room.
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                if let Ok(stats) = service.stats_for("self").await {
                    assert!(stats.memory_usage_percent() <= 100.0);
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("local node should report within 5s");
        service.shutdown();
    }
}
