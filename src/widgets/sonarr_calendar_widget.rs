use async_trait::async_trait;
use chrono::{DateTime, Duration, Local, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashSet;

use crate::error::VestaResult;
use crate::http_client::send_checked_json;
use crate::widget_system::{WidgetContext, WidgetField, WidgetHandler, WidgetQuery};

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    /// Sonarr base URL, e.g. `https://sonarr.example.com`.
    pub url: String,
    /// Sonarr API key.
    pub key: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Series {
    title: String,
    #[serde(alias = "titleSlug", rename(serialize = "titleSlug"))]
    title_slug: String,
    runtime: i64,
}

#[derive(Serialize, Deserialize, Debug)]
struct CalendarEntry {
    #[serde(alias = "seriesId", rename(serialize = "seriesId"))]
    series_id: u32,
    #[serde(alias = "seasonNumber", rename(serialize = "seasonNumber"))]
    season_number: u8,
    #[serde(alias = "episodeNumber", rename(serialize = "episodeNumber"))]
    episode_number: u32,
    title: Option<String>,
    #[serde(alias = "airDateUtc", rename(serialize = "airDateUtc"))]
    air_date_utc: DateTime<Utc>,
    series: Series,
    #[serde(alias = "hasFile", rename(serialize = "hasFile"))]
    has_file: bool,
    monitored: bool,
    #[serde(
        skip_deserializing,
        alias = "isDownloading",
        rename(serialize = "isDownloading")
    )]
    is_downloading: bool,
}

type Calendar = Vec<CalendarEntry>;

#[derive(Deserialize, Debug)]
struct DownloadRecord {
    #[serde(alias = "seriesId", rename(serialize = "seriesId"))]
    series_id: u32,
}

#[derive(Deserialize, Debug)]
struct DownloadQueue {
    records: Vec<DownloadRecord>,
}

pub struct SonarrCalendarWidget {
    client: Client,
}

impl SonarrCalendarWidget {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    async fn fetch_calendar(&self, config: &Config) -> VestaResult<Calendar> {
        let today = Utc::now();
        let day_after_tomorrow = today + Duration::days(2);
        let params = [
            ("unmonitored", "false"),
            ("includeSeries", "true"),
            ("start", &today.format("%Y-%m-%d").to_string()),
            ("end", &day_after_tomorrow.format("%Y-%m-%d").to_string()),
        ];

        let base = config.url.trim_end_matches('/');
        send_checked_json(
            self.client
                .get(format!("{base}/api/v3/calendar"))
                .query(&params)
                .header("X-Api-Key", &config.key),
            "Sonarr calendar",
        )
        .await
    }

    async fn fetch_download_queue(&self, config: &Config) -> VestaResult<DownloadQueue> {
        let base = config.url.trim_end_matches('/');
        send_checked_json(
            self.client
                .get(format!("{base}/api/v3/queue"))
                .header("X-Api-Key", &config.key),
            "Sonarr download queue",
        )
        .await
    }
}

fn format_time(date: &DateTime<Utc>) -> String {
    date.with_timezone(&Local).format("%H:%M").to_string()
}

fn format_air_time(entry: &CalendarEntry) -> String {
    let end = entry.air_date_utc + Duration::minutes(entry.series.runtime);
    format!(
        "{} - {}",
        format_time(&entry.air_date_utc),
        format_time(&end)
    )
}

fn entry_class(entry: &CalendarEntry, now: &DateTime<Utc>) -> &'static str {
    let aired_at = entry.air_date_utc + Duration::minutes(entry.series.runtime);
    if *now < entry.air_date_utc {
        "unaired"
    } else if entry.is_downloading {
        "downloading"
    } else if entry.has_file {
        "downloaded"
    } else if *now <= aired_at {
        "airing"
    } else {
        "missing"
    }
}

#[async_trait]
impl WidgetHandler for SonarrCalendarWidget {
    fn name(&self) -> &str {
        "SonarrCalendar"
    }

    fn description(&self) -> &str {
        "Shows upcoming Sonarr episodes and download status."
    }

    fn default_size(&self) -> (u8, u8) {
        (4, 4)
    }

    fn min_size(&self) -> (u8, u8) {
        (3, 3)
    }

    fn fields(&self) -> Vec<WidgetField> {
        vec![
            WidgetField::builtin(
                "url",
                "Sonarr URL",
                "url",
                true,
                "The base URL of your Sonarr server.",
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

    type Config = Config;

    async fn handle_request(
        &self,
        _ctx: WidgetContext,
        _query: WidgetQuery,
        config: Config,
    ) -> VestaResult<Value> {
        let (calendar, queue) = tokio::join!(
            self.fetch_calendar(&config),
            self.fetch_download_queue(&config),
        );
        let mut calendar = calendar?;
        let downloading: HashSet<u32> = queue?.records.into_iter().map(|r| r.series_id).collect();

        for entry in &mut calendar {
            entry.is_downloading = downloading.contains(&entry.series_id);
        }

        let base = config.url.trim_end_matches('/');
        let now = Utc::now();
        let entries = calendar
            .iter()
            .map(|entry| {
                json!({
                    "date": entry.air_date_utc.with_timezone(&Local).format("%Y-%m-%d").to_string(),
                    "series_url": format!("{base}/series/{}", entry.series.title_slug),
                    "title": entry.series.title,
                    "season_number": entry.season_number,
                    "episode_number": entry.episode_number,
                    "air_time": format_air_time(entry),
                    "status": entry_class(entry, &now),
                })
            })
            .collect::<Vec<_>>();

        Ok(json!({
            "calendar_url": format!("{base}/calendar"),
            "entries": entries,
        }))
    }
}

// CalendarEntry carries series_id from the API but matches on the nested
// series.id is unnecessary: the API's seriesId equals series.id, so the field
// is kept deserialized for the queue join above.
