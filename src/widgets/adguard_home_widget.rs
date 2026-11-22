use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;

use crate::error::VestaResult;
use crate::http_client::{join_url, parse_logged, send_checked_json};
use crate::widget_system::{
    WidgetContext, WidgetField, WidgetHandler, WidgetQuery, refresh_interval_field,
};

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub url: String,
    pub username: String,
    pub password: String,
    #[serde(
        default = "default_refresh",
        deserialize_with = "crate::widget_system::debounced_u64"
    )]
    pub refresh_interval: u64,
}

fn default_refresh() -> u64 {
    300
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
enum TopEntry {
    Object {
        domain: String,
        #[serde(default)]
        count: u64,
    },
    Pair(u64, String),
    Map(HashMap<String, u64>),
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(untagged)]
enum TopDomains {
    List(Vec<TopEntry>),
    Map(HashMap<String, u64>),
    #[default]
    Missing,
}

impl TopDomains {
    fn rows(&self) -> Vec<(String, u64)> {
        match self {
            TopDomains::List(entries) => entries
                .iter()
                .flat_map(|entry| match entry {
                    TopEntry::Object { domain, count } => vec![(domain.clone(), *count)],
                    TopEntry::Pair(count, domain) => vec![(domain.clone(), *count)],
                    TopEntry::Map(map) => map
                        .iter()
                        .map(|(domain, count)| (domain.clone(), *count))
                        .collect::<Vec<_>>(),
                })
                .collect(),
            TopDomains::Map(domains) => domains
                .iter()
                .map(|(domain, count)| (domain.clone(), *count))
                .collect(),
            TopDomains::Missing => Vec::new(),
        }
    }
}

fn top_blocked_domains(domains: &TopDomains, limit: usize) -> Vec<Value> {
    let mut rows = domains.rows();
    rows.sort_by_key(|row| (std::cmp::Reverse(row.1), row.0.clone()));
    rows.into_iter()
        .take(limit)
        .map(|(domain, count)| json!({ "domain": domain, "count": count }))
        .collect()
}

fn blocked_percent(blocked: u64, queries: u64) -> f64 {
    if queries == 0 {
        0.0
    } else {
        blocked as f64 / queries as f64 * 100.0
    }
}

#[derive(Deserialize, Debug, Default)]
struct StatsResponse {
    #[serde(default)]
    num_dns_queries: u64,
    #[serde(default)]
    num_blocked_filtering: u64,
    #[serde(default)]
    num_replaced_safebrowsing: u64,
    #[serde(default)]
    num_replaced_parental: u64,
    #[serde(default)]
    avg_processing_time: f64,
    #[serde(default)]
    top_blocked_domains: TopDomains,
}

#[derive(Deserialize, Debug, Default)]
struct StatusResponse {
    #[serde(default)]
    version: String,
    #[serde(default)]
    protection_enabled: bool,
    #[serde(default)]
    running: bool,
}

#[derive(Deserialize, Debug, Default)]
struct FilteringStatus {
    #[serde(default)]
    filters: Vec<FilterEntry>,
}

#[derive(Deserialize, Debug, Default)]
struct FilterEntry {
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    rules_count: u64,
}

pub struct AdGuardHomeWidget {
    client: reqwest::Client,
}

impl AdGuardHomeWidget {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }

    async fn fetch_json(&self, path: &str, username: &str, password: &str) -> VestaResult<Value> {
        let error_path = path.to_string();
        send_checked_json(
            self.client.get(path).basic_auth(username, Some(password)),
            move |status| format!("AdGuard Home request to {error_path} failed: {status}"),
        )
        .await
    }
}

fn tile_payload(stats: StatsResponse, status: StatusResponse, filtering: FilteringStatus) -> Value {
    let filters_count = filtering.filters.len();
    let rules_count: u64 = filtering
        .filters
        .iter()
        .filter(|filter| filter.enabled)
        .map(|filter| filter.rules_count)
        .sum();

    json!({
        "num_dns_queries": stats.num_dns_queries,
        "num_blocked_filtering": stats.num_blocked_filtering,
        "blocked_percent": blocked_percent(stats.num_blocked_filtering, stats.num_dns_queries),
        "num_replaced_parental": stats.num_replaced_parental,
        "num_replaced_safebrowsing": stats.num_replaced_safebrowsing,
        "avg_processing_time": stats.avg_processing_time,
        "top_blocked_domains": top_blocked_domains(&stats.top_blocked_domains, 3),
        "version": status.version,
        "protection_enabled": status.protection_enabled,
        "running": status.running,
        "filters_count": filters_count,
        "rules_count": rules_count,
    })
}

#[async_trait]
impl WidgetHandler for AdGuardHomeWidget {
    fn name(&self) -> &str {
        "AdGuard"
    }

    fn title(&self) -> String {
        "AdGuard Home".to_string()
    }

    fn description(&self) -> &str {
        "Shows AdGuard Home DNS query statistics and protection status."
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
                "AdGuard Home URL",
                "url",
                true,
                "The base URL of your AdGuard Home instance.",
            ),
            WidgetField::builtin(
                "username",
                "Username",
                "text",
                true,
                "The username you sign in to AdGuard Home with.",
            ),
            WidgetField::builtin(
                "password",
                "Password",
                "password",
                true,
                "Stored in config.toml; protect that file as a secret.",
            ),
            refresh_interval_field(300),
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
        let stats_url = join_url(&config.url, "/control/stats");
        let status_url = join_url(&config.url, "/control/status");
        let filtering_url = join_url(&config.url, "/control/filtering/status");
        let (stats, status, filtering) = tokio::join!(
            self.fetch_json(&stats_url, &config.username, &config.password),
            self.fetch_json(&status_url, &config.username, &config.password),
            self.fetch_json(&filtering_url, &config.username, &config.password),
        );
        let stats: StatsResponse = parse_logged("AdGuard", "/control/stats", &stats?)?;
        let status: StatusResponse = parse_logged("AdGuard", "/control/status", &status?)?;
        let filtering: FilteringStatus =
            parse_logged("AdGuard", "/control/filtering/status", &filtering?)?;

        Ok(tile_payload(stats, status, filtering))
    }
}

#[cfg(test)]
mod tests {
    use super::{FilteringStatus, StatsResponse, StatusResponse, TopDomains, TopEntry};
    use super::{blocked_percent, tile_payload, top_blocked_domains};
    use serde_json::json;

    #[test]
    fn parses_object_top_domains() {
        let stats: StatsResponse = serde_json::from_value(json!({
            "num_dns_queries": 123_456,
            "num_blocked_filtering": 12_345,
            "avg_processing_time": 0.041,
            "top_blocked_domains": [
                { "domain": "ads.example.com", "count": 9_000 },
                { "domain": "tracker.example.net", "count": 2_000 }
            ]
        }))
        .unwrap();
        assert_eq!(stats.num_dns_queries, 123_456);
        assert_eq!(stats.num_blocked_filtering, 12_345);
        assert_eq!(stats.top_blocked_domains.rows().len(), 2);
    }

    #[test]
    fn parses_pair_top_domains() {
        let stats: StatsResponse = serde_json::from_value(json!({
            "num_dns_queries": 10,
            "top_blocked_domains": [[5, "ads.example.com"], [1, "ads2.example.com"]]
        }))
        .unwrap();
        assert_eq!(
            stats.top_blocked_domains.rows()[0],
            ("ads.example.com".to_string(), 5)
        );
        assert_eq!(
            stats.top_blocked_domains.rows()[1],
            ("ads2.example.com".to_string(), 1)
        );
    }

    #[test]
    fn parses_map_top_domains() {
        let stats: StatsResponse = serde_json::from_value(json!({
            "num_dns_queries": 10,
            "top_blocked_domains": { "ads.example.com": 5, "ads2.example.com": 1 }
        }))
        .unwrap();
        let mut rows = stats.top_blocked_domains.rows();
        rows.sort_by_key(|row| std::cmp::Reverse(row.1));
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], ("ads.example.com".to_string(), 5));
    }

    #[test]
    fn top_blocked_domains_ranks_and_caps() {
        let real_stats: StatsResponse = serde_json::from_value(json!({
            "time_units": "hours",
            "num_dns_queries": 116497,
            "num_blocked_filtering": 14352,
            "num_replaced_safebrowsing": 0,
            "num_replaced_safesearch": 0,
            "num_replaced_parental": 0,
            "avg_processing_time": 0.0128,
            "dns_queries": [100, 120],
            "blocked_filtering": [5, 6],
            "replaced_safebrowsing": [0, 0],
            "replaced_parental": [0, 0],
            "top_queried_domains": [{ "chatgpt.com": 26794 }, { "api.mistral.ai": 16466 }],
            "top_blocked_domains": [{ "stats.grafana.org": 14352 }, { "nrdp.logs.netflix.com": 1306 }],
            "top_clients": [{ "192.168.1.6": 116497 }, { "192.168.1.4": 41209 }]
        }))
        .expect("the real 0.107.x payload must parse");
        let blocked = top_blocked_domains(&real_stats.top_blocked_domains, 3);
        assert_eq!(blocked[0]["domain"], json!("stats.grafana.org"));
        assert_eq!(blocked[0]["count"], json!(14352));
        assert_eq!(blocked[1]["domain"], json!("nrdp.logs.netflix.com"));

        let domains = TopDomains::List(vec![
            TopEntry::Object {
                domain: "low.example.com".to_string(),
                count: 1,
            },
            TopEntry::Object {
                domain: "high.example.com".to_string(),
                count: 9,
            },
            TopEntry::Object {
                domain: "mid.example.com".to_string(),
                count: 4,
            },
            TopEntry::Object {
                domain: "cut.example.com".to_string(),
                count: 2,
            },
        ]);
        let rows = top_blocked_domains(&domains, 3);
        let names: Vec<&str> = rows
            .iter()
            .map(|row| row["domain"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            vec!["high.example.com", "mid.example.com", "cut.example.com"]
        );
        assert_eq!(rows[0]["count"], json!(9));
        let empty = top_blocked_domains(&TopDomains::Missing, 3);
        assert!(empty.is_empty());
    }

    #[test]
    fn blocked_percent_handles_zero_queries() {
        assert_eq!(blocked_percent(0, 0), 0.0);
        assert_eq!(blocked_percent(25, 0), 0.0);
        assert!((blocked_percent(1, 4) - 25.0).abs() < f64::EPSILON);
        assert!((blocked_percent(3, 4) - 75.0).abs() < f64::EPSILON);
    }

    #[test]
    fn tile_payload_sums_filter_rules() {
        let filtering: FilteringStatus = serde_json::from_value(json!({
            "filters": [
                { "enabled": true, "rules_count": 100 },
                { "enabled": false, "rules_count": 500 }
            ]
        }))
        .unwrap();
        let status: StatusResponse = serde_json::from_value(json!({
            "version": "0.107.44",
            "protection_enabled": true
        }))
        .unwrap();
        let stats: StatsResponse = serde_json::from_value(json!({
            "num_dns_queries": 400,
            "num_blocked_filtering": 100,
            "avg_processing_time": 0.05
        }))
        .unwrap();

        let payload = tile_payload(stats, status, filtering);
        assert_eq!(payload["num_dns_queries"], json!(400));
        assert_eq!(payload["num_blocked_filtering"], json!(100));
        assert_eq!(payload["blocked_percent"], json!(25.0));
        assert_eq!(payload["filters_count"], json!(2));
        assert_eq!(payload["rules_count"], json!(100));
        assert_eq!(payload["version"], json!("0.107.44"));
        assert_eq!(payload["protection_enabled"], json!(true));
        assert_eq!(payload["top_blocked_domains"], json!([]));
    }

    #[test]
    fn missing_fields_fail_soft() {
        let status: StatusResponse = serde_json::from_value(json!({ "version": "0.107" })).unwrap();
        assert!(!status.protection_enabled);
        assert!(!status.running);
        let stats: StatsResponse = serde_json::from_value(json!({})).unwrap();
        assert_eq!(stats.num_dns_queries, 0);
        assert_eq!(stats.avg_processing_time, 0.0);
    }
}
