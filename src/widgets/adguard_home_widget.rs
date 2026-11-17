use async_trait::async_trait;
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::collections::HashMap;

use crate::error::{VestaError, VestaResult};
use crate::widget_system::{WidgetContext, WidgetField, WidgetHandler, WidgetQuery};

/// AdGuard Home widget.
#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    /// AdGuard Home base URL, e.g. `http://adguard.example.com`.
    pub url: String,
    /// Basic-auth username shown on the AdGuard Home login.
    pub username: String,
    /// Basic-auth password. Stored in config.toml; protect that file as a
    /// secret.
    pub password: String,
    #[serde(
        default = "default_refresh",
        deserialize_with = "crate::widget_config::debounced_u64"
    )]
    pub refresh_interval: u64,
}

fn default_refresh() -> u64 {
    300
}

/// `top_*_domains` entries: newer AdGuard versions expose objects with
/// `domain` and `count` fields; older versions return `[count, domain]`.
#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
enum TopEntry {
    Object {
        domain: String,
        #[serde(default)]
        count: u64,
    },
    Pair(u64, String),
    /// Single-key `{domain: count}` entries, the shape AdGuard Home 0.107.x
    /// actually serves for `top_blocked_domains`.
    Map(HashMap<String, u64>),
}

/// The `top_blocked_domains` collections returned by `/control/stats`:
/// a list of entries (either shape) or a map keyed by domain.
#[derive(Deserialize, Debug, Clone, Default)]
#[serde(untagged)]
enum TopDomains {
    List(Vec<TopEntry>),
    Map(HashMap<String, u64>),
    #[default]
    Missing,
}

impl TopDomains {
    /// Normalise every accepted shape into `(domain, count)` rows.
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

/// Top blocked domains, highest count first, capped at `limit` rows.
fn top_blocked_domains(domains: &TopDomains, limit: usize) -> Vec<Value> {
    let mut rows = domains.rows();
    rows.sort_by_key(|row| (std::cmp::Reverse(row.1), row.0.clone()));
    rows.into_iter()
        .take(limit)
        .map(|(domain, count)| json!({ "domain": domain, "count": count }))
        .collect()
}

/// Percentage of DNS queries blocked by filtering; `0.0` when no queries
/// were seen to avoid dividing by zero.
fn blocked_percent(blocked: u64, queries: u64) -> f64 {
    if queries == 0 {
        0.0
    } else {
        blocked as f64 / queries as f64 * 100.0
    }
}

/// Shape of `/control/stats`. Unknown fields are ignored; counters that
/// AdGuard omits report zero so one odd field never fails the tile.
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

/// Shape of `/control/status`.
#[derive(Deserialize, Debug, Default)]
struct StatusResponse {
    #[serde(default)]
    version: String,
    #[serde(default)]
    protection_enabled: bool,
    #[serde(default)]
    running: bool,
}

/// Shape of `/control/filtering/status`.
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
        let response = self
            .client
            .get(path)
            .basic_auth(username, Some(password))
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            return Err(VestaError::ApiError {
                status,
                message: format!("AdGuard Home request to {path} failed: {status}"),
            });
        }
        Ok(response.json::<Value>().await?)
    }
}

/// Parse a widget payload, logging a truncated raw copy on shape mismatch:
/// payload drift between AdGuard versions is the likely failure here, and
/// the log removes the guesswork when a version changes shape again.
fn parse_logged<T: DeserializeOwned>(endpoint: &str, value: &Value) -> VestaResult<T> {
    serde_json::from_value(value.clone()).map_err(|error| {
        let payload: String = value.to_string().chars().take(512).collect();
        tracing::warn!(
            target: "vesta::widgets::adguard_home",
            endpoint,
            %error,
            payload,
            "AdGuard payload did not match the expected shape"
        );
        VestaError::ApiError {
            status: axum::http::StatusCode::BAD_GATEWAY,
            message: format!(
                "AdGuard {endpoint} payload did not match the expected shape: {error}"
            ),
        }
    })
}

/// Assemble the tile payload from the three AdGuard API responses.
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
    fn name(&self) -> &'static str {
        "AdGuard"
    }

    fn title(&self) -> String {
        "AdGuard Home".to_string()
    }

    fn description(&self) -> &'static str {
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
            WidgetField {
                key: "url",
                label: "AdGuard Home URL",
                kind: "url",
                required: true,
                description: "The base URL of your AdGuard Home instance.",
                default: None,
                options: Vec::new(),
            },
            WidgetField {
                key: "username",
                label: "Username",
                kind: "text",
                required: true,
                description: "The username you sign in to AdGuard Home with.",
                default: None,
                options: Vec::new(),
            },
            WidgetField {
                key: "password",
                label: "Password",
                kind: "password",
                required: true,
                description: "Stored in config.toml; protect that file as a secret.",
                default: None,
                options: Vec::new(),
            },
            WidgetField {
                key: "refresh_interval",
                label: "Refresh every (seconds)",
                kind: "number",
                required: true,
                description: "How often this widget requests updated values (1–3600 seconds).",
                default: Some(serde_json::json!(300)),
                options: Vec::new(),
            },
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
        let stats_url = format!("{base}/control/stats");
        let status_url = format!("{base}/control/status");
        let filtering_url = format!("{base}/control/filtering/status");
        let (stats, status, filtering) = tokio::join!(
            self.fetch_json(&stats_url, &config.username, &config.password),
            self.fetch_json(&status_url, &config.username, &config.password),
            self.fetch_json(&filtering_url, &config.username, &config.password),
        );
        let stats: StatsResponse = parse_logged("/control/stats", &stats?)?;
        let status: StatusResponse = parse_logged("/control/status", &status?)?;
        let filtering: FilteringStatus = parse_logged("/control/filtering/status", &filtering?)?;

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
        // The real /control/stats payload (AdGuard Home 0.107.x) carries
        // top_blocked_domains as a list of single-key {domain: count} maps.
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
