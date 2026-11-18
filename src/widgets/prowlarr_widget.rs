use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;

use crate::error::VestaResult;
use crate::http_client::{parse_logged, send_checked_json};
use crate::widget_system::{WidgetContext, WidgetField, WidgetHandler, WidgetQuery};

/// Prowlarr indexer widget.
#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    /// Prowlarr base URL, e.g. `https://prowlarr.example.com`.
    pub url: String,
    /// Prowlarr API key.
    pub key: String,
    #[serde(
        default = "default_refresh",
        deserialize_with = "crate::widget_config::debounced_u64"
    )]
    pub refresh_interval: u64,
}

fn default_refresh() -> u64 {
    300
}

#[derive(Deserialize, Debug)]
struct IndexerEntry {
    #[serde(default)]
    id: Option<u64>,
    name: String,
    #[serde(default = "default_true")]
    enable: bool,
}

fn default_true() -> bool {
    true
}

/// Response row of `/api/v1/indexerstats`. Current Prowlarr names the
/// counters `numberOfQueries`/`numberOfGrabs`; older generations used
/// `queries`/`grabs` — both are accepted.
#[derive(Clone, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
struct IndexerStatsEntry {
    #[serde(default)]
    indexer_id: Option<u64>,
    #[serde(default)]
    indexer_name: String,
    #[serde(default, alias = "queries")]
    number_of_queries: u64,
    #[serde(default, alias = "grabs")]
    number_of_grabs: u64,
    #[serde(default, alias = "rssQueries")]
    number_of_rss_queries: u64,
    #[serde(default, alias = "failedQueries")]
    number_of_failed_queries: u64,
    #[serde(default, alias = "failedGrabs")]
    number_of_failed_grabs: u64,
    #[serde(default)]
    average_response_time: Option<f64>,
}

/// `/api/v1/indexerstats` wraps the rows in an object; a bare list from
/// older builds is also accepted.
#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum IndexerStatsShape {
    Wrapped {
        #[serde(alias = "indexerStatsResponse")]
        indexers: Vec<IndexerStatsEntry>,
    },
    Bare(Vec<IndexerStatsEntry>),
}

#[derive(Serialize, Debug)]
struct IndexerRow {
    name: String,
    enable: bool,
    queries: u64,
    grabs: u64,
    rss_queries: u64,
    failed_queries: u64,
    failed_grabs: u64,
    average_response_time: Option<f64>,
}

pub struct ProwlarrWidget {
    client: reqwest::Client,
}

impl ProwlarrWidget {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }

    async fn fetch_json(&self, url: String, key: &str) -> VestaResult<Value> {
        send_checked_json(
            self.client.get(&url).header("X-Api-Key", key),
            &format!("Prowlarr {url}"),
        )
        .await
    }

    /// Parse `/api/v1/indexerstats` for the per-indexer rows.
    fn parse_stats(&self, value: &Value) -> VestaResult<Vec<IndexerStatsEntry>> {
        let shape: IndexerStatsShape = parse_logged("Prowlarr", "/api/v1/indexerstats", value)?;
        Ok(match shape {
            IndexerStatsShape::Wrapped { indexers } => indexers,
            IndexerStatsShape::Bare(stats) => stats,
        })
    }
}

/// Merge indexer definitions with per-indexer usage stats. Stats rows are
/// matched to indexers by id when both sides have one, otherwise by name;
/// indexers missing from the stats response report zero usage.
fn indexer_rows(indexers: Vec<IndexerEntry>, stats: Vec<IndexerStatsEntry>) -> Vec<IndexerRow> {
    let mut stats_by_id: HashMap<u64, &IndexerStatsEntry> = HashMap::new();
    let mut stats_by_name: HashMap<String, &IndexerStatsEntry> = HashMap::new();
    for entry in stats.iter() {
        if let Some(id) = entry.indexer_id {
            stats_by_id.insert(id, entry);
        }
        if !entry.indexer_name.is_empty() {
            stats_by_name.insert(entry.indexer_name.clone(), entry);
        }
    }

    indexers
        .iter()
        .map(|indexer| {
            let usage = indexer
                .id
                .and_then(|id| stats_by_id.get(&id))
                .or_else(|| stats_by_name.get(&indexer.name))
                .copied();
            let usage = usage.cloned().unwrap_or_default();
            IndexerRow {
                name: indexer.name.clone(),
                enable: indexer.enable,
                queries: usage.number_of_queries,
                grabs: usage.number_of_grabs,
                rss_queries: usage.number_of_rss_queries,
                failed_queries: usage.number_of_failed_queries,
                failed_grabs: usage.number_of_failed_grabs,
                average_response_time: usage.average_response_time,
            }
        })
        .collect()
}

/// Roll per-indexer usage up into tile totals; `None` when no indexer
/// reports an average response time.
fn aggregate(rows: &[IndexerRow]) -> (u64, u64, Option<f64>) {
    let mut queries = 0;
    let mut grabs = 0;
    let mut response_times = Vec::new();
    for row in rows {
        queries += row.queries;
        grabs += row.grabs;
        if let Some(time) = row.average_response_time {
            response_times.push(time);
        }
    }
    let average = if response_times.is_empty() {
        None
    } else {
        Some(response_times.iter().sum::<f64>() / response_times.len() as f64)
    };
    (queries, grabs, average)
}

#[async_trait]
impl WidgetHandler for ProwlarrWidget {
    fn name(&self) -> &str {
        "Prowlarr"
    }

    fn description(&self) -> &str {
        "Shows Prowlarr indexer status and query statistics."
    }

    fn default_size(&self) -> (u8, u8) {
        (4, 2)
    }

    fn min_size(&self) -> (u8, u8) {
        (3, 2)
    }

    type Config = Config;

    fn fields(&self) -> Vec<WidgetField> {
        vec![
            WidgetField::builtin(
                "url",
                "Prowlarr URL",
                "url",
                true,
                "The base URL of your Prowlarr instance.",
            ),
            WidgetField::builtin(
                "key",
                "API key",
                "password",
                true,
                "Stored in config.toml; protect that file as a secret.",
            ),
        ]
    }

    fn refresh_interval_secs(&self, config: &Config) -> Option<u64> {
        Some(config.refresh_interval)
    }

    async fn handle_request(
        &self,
        _ctx: WidgetContext,
        _query: WidgetQuery,
        config: Config,
    ) -> VestaResult<Value> {
        let base = config.url.trim_end_matches('/');
        let key = config.key;
        let (indexers_value, stats_value) = tokio::join!(
            self.fetch_json(format!("{base}/api/v1/indexer"), &key),
            self.fetch_json(format!("{base}/api/v1/indexerstats"), &key),
        );
        let indexers_value = indexers_value?;
        let stats_value = stats_value?;

        let indexers: Vec<IndexerEntry> =
            parse_logged("Prowlarr", "/api/v1/indexer", &indexers_value)?;
        let stats = self.parse_stats(&stats_value)?;
        let rows = indexer_rows(indexers, stats);
        let (queries, grabs, average_response_time) = aggregate(&rows);

        Ok(json!({
            "indexers": rows,
            "totals": {
                "queries": queries,
                "grabs": grabs,
                "average_response_time": average_response_time,
            },
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::{IndexerEntry, IndexerStatsEntry, IndexerStatsShape, aggregate, indexer_rows};
    use serde_json::json;

    fn indexer(id: Option<u64>, name: &str, enable: bool) -> IndexerEntry {
        IndexerEntry {
            id,
            name: name.to_string(),
            enable,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn stats(
        indexer_id: Option<u64>,
        indexer_name: &str,
        queries: u64,
        grabs: u64,
        failed_queries: u64,
        failed_grabs: u64,
    ) -> IndexerStatsEntry {
        IndexerStatsEntry {
            indexer_id,
            indexer_name: indexer_name.to_string(),
            number_of_queries: queries,
            number_of_grabs: grabs,
            number_of_rss_queries: 0,
            number_of_failed_queries: failed_queries,
            number_of_failed_grabs: failed_grabs,
            average_response_time: None,
        }
    }

    /// The real `/api/v1/indexerstats` payload (captured live from Prowlarr
    /// 3.x): a wrapper object whose `indexers` key holds rows using the
    /// `numberOf*` counter names.
    #[test]
    fn parses_the_live_wrapped_payload() {
        let shape: super::IndexerStatsShape = serde_json::from_value(json!({
            "indexers": [
                {
                    "indexerId": 4,
                    "indexerName": "Shana Project",
                    "averageResponseTime": 865,
                    "averageGrabResponseTime": 0,
                    "numberOfQueries": 4,
                    "numberOfGrabs": 0,
                    "numberOfRssQueries": 1194,
                    "numberOfAuthQueries": 0,
                    "numberOfFailedQueries": 0,
                    "numberOfFailedGrabs": 0,
                    "numberOfFailedRssQueries": 0,
                    "numberOfFailedAuthQueries": 0
                }
            ],
            "userAgents": [],
            "hosts": []
        }))
        .expect("the live 3.x payload must parse");
        let IndexerStatsShape::Wrapped { indexers } = shape else {
            panic!("the live payload uses the wrapped shape");
        };
        assert_eq!(indexers[0].indexer_name, "Shana Project");
        assert_eq!(indexers[0].number_of_queries, 4);
        assert_eq!(indexers[0].number_of_rss_queries, 1194);
        assert_eq!(indexers[0].average_response_time, Some(865.0));
    }

    /// Older Prowlarr generations served a bare array with `queries` /
    /// `failedQueries`-style names.
    #[test]
    fn parses_legacy_bare_arrays() {
        let stats: Vec<IndexerStatsEntry> = serde_json::from_value(json!([
            {
                "indexerId": 1,
                "indexerName": "Example Indexer",
                "averageResponseTime": 212,
                "queries": 40,
                "grabs": 12,
                "rssQueries": 30,
                "authQueries": 10,
                "failedQueries": 1,
                "failedGrabs": 0,
                "failedRssQueries": 0,
                "failedAuthQueries": 0
            }
        ]))
        .unwrap();
        assert_eq!(stats[0].number_of_queries, 40);
        assert_eq!(stats[0].number_of_failed_queries, 1);
        assert_eq!(stats[0].average_response_time, Some(212.0));
    }

    #[test]
    fn end_to_end_merge_uses_live_counter_names() {
        let indexers: Vec<IndexerEntry> = serde_json::from_value(json!([
            { "id": 4, "name": "Shana Project", "enable": true }
        ]))
        .unwrap();
        let rows = indexer_rows(
            indexers,
            vec![IndexerStatsEntry {
                indexer_id: Some(4),
                indexer_name: "Shana Project".to_string(),
                number_of_queries: 4,
                number_of_grabs: 0,
                number_of_rss_queries: 1194,
                number_of_failed_queries: 0,
                number_of_failed_grabs: 0,
                average_response_time: Some(865.0),
            }],
        );
        assert_eq!(rows[0].name, "Shana Project");
        assert_eq!(rows[0].queries, 4);
        assert_eq!(rows[0].rss_queries, 1194);
        let (queries, grabs, avg) = aggregate(&rows);
        assert_eq!((queries, grabs), (4, 0));
        assert_eq!(avg, Some(865.0));
    }

    #[test]
    fn rows_match_stats_by_id_then_name() {
        let indexers = vec![
            indexer(Some(1), "Alpha", true),
            indexer(Some(2), "NoStats", true),
        ];
        let rows_list = indexer_rows(indexers, vec![stats(Some(1), "DifferentName", 4, 2, 0, 0)]);
        assert_eq!(rows_list.len(), 2);
        assert_eq!(rows_list[0].queries, 4);
        assert_eq!(rows_list[1].queries, 0);
    }

    #[test]
    fn statuses_follow_enable_and_failures() {
        let indexers = vec![
            indexer(Some(1), "Enabled", true),
            indexer(Some(2), "Disabled", false),
            indexer(Some(3), "Flaky", true),
        ];
        let stats = vec![
            stats(Some(1), "Enabled", 5, 1, 0, 0),
            stats(Some(2), "Disabled", 1, 0, 0, 0),
            stats(Some(3), "Flaky", 1, 1, 3, 1),
        ];
        let rows = indexer_rows(indexers, stats);
        assert_eq!(rows[0].name, "Enabled");
        assert!(!rows[1].enable);
        assert_eq!(rows[2].failed_queries, 3);
    }

    #[test]
    fn aggregate_returns_none_average_without_response_times() {
        let indexers = vec![indexer(Some(1), "Only", true)];
        let rows = indexer_rows(indexers, vec![stats(Some(1), "Only", 2, 1, 0, 0)]);
        let result = aggregate(&rows);
        assert_eq!(result.0, 2);
        assert_eq!(result.1, 1);
        assert_eq!(aggregate(&[]), (0, 0, None));
    }
}
