use async_trait::async_trait;
use chrono::{DateTime, Duration, Local, Utc};
use indexmap::IndexMap;
use maud::{html, Markup};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::Arc};

use crate::{
    config::{Service, Widget},
    error::{VestaError, VestaResult},
    widget_system::{WidgetHandler, WidgetQuery},
    widgets::widget_container,
    AppState,
};

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
    // custom field
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

pub struct SonarrCalendarWidget;

impl SonarrCalendarWidget {
    pub fn new() -> Self {
        Self
    }

    async fn fetch_download_queue(
        &self,
        client: &Client,
        url: &str,
        key: &str,
    ) -> VestaResult<DownloadQueue> {
        let response = client
            .get(format!("{}/api/v3/queue", url))
            .header("X-Api-Key", key)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(VestaError::ApiError {
                status: response.status(),
                message: "Failed to fetch download queue".to_string(),
            });
        }

        let download_queue = response.json::<DownloadQueue>().await?;
        Ok(download_queue)
    }

    async fn fetch_calendar(&self, client: &Client, url: &str, key: &str) -> VestaResult<Calendar> {
        let today = Utc::now();
        let day_after_tomorrow = today + Duration::days(2);
        let params = [
            ("unmonitored", "false"),
            ("includeSeries", "true"),
            ("start", &today.format("%Y-%m-%d").to_string()),
            ("end", &day_after_tomorrow.format("%Y-%m-%d").to_string()),
        ];

        let response = client
            .get(format!("{}/api/v3/calendar", url))
            .query(&params)
            .header("X-Api-Key", key)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(VestaError::ApiError {
                status: response.status(),
                message: "Failed to fetch calendar".to_string(),
            });
        }

        let calendar = response.json::<Calendar>().await?;
        Ok(calendar)
    }

    fn get_widget_credentials<'a>(
        &self,
        widget_info: &'a Widget,
    ) -> VestaResult<(&'a str, &'a str)> {
        widget_info
            .config
            .as_ref()
            .and_then(|config| {
                let url = config.get("url")?;
                let key = config.get("key")?;
                Some((url.as_str(), key.as_str()))
            })
            .ok_or_else(|| VestaError::MissingCredentials {
                field: "url or key".to_string(),
            })
    }

    fn format_time(&self, date: &DateTime<Utc>) -> String {
        date.with_timezone(&Local).format("%H:%M").to_string()
    }

    fn add_minutes(&self, date: &DateTime<Utc>, minutes: i64) -> DateTime<Utc> {
        *date + Duration::minutes(minutes)
    }

    fn format_air_time(&self, entry: &CalendarEntry) -> String {
        let air_date = self.format_time(&entry.air_date_utc);
        let aired_date =
            self.format_time(&self.add_minutes(&entry.air_date_utc, entry.series.runtime));
        format!("{} - {}", air_date, aired_date)
    }

    fn format_episode(&self, entry: &CalendarEntry) -> String {
        format!("{}x{:02}", entry.season_number, entry.episode_number)
    }

    fn format_series_url(&self, sonarr_url: &str, title_slug: &str) -> String {
        format!("{}/series/{}", sonarr_url, title_slug)
    }

    fn get_entry_class(&self, entry: &CalendarEntry, current_date: &DateTime<Utc>) -> &'static str {
        let aired_date = self.add_minutes(&entry.air_date_utc, entry.series.runtime);

        if *current_date < entry.air_date_utc {
            "unaired"
        } else if entry.is_downloading {
            "downloading"
        } else if entry.has_file {
            "downloaded"
        } else if *current_date >= entry.air_date_utc && *current_date <= aired_date {
            "airing"
        } else if !entry.has_file && *current_date > aired_date {
            "missing"
        } else {
            ""
        }
    }
}

#[async_trait]
impl WidgetHandler for SonarrCalendarWidget {
    fn name(&self) -> &'static str {
        "SonarrCalendar"
    }

    fn render(&self, group_id: &str, service: &Service) -> Markup {
        widget_container(
            service.width,
            service.height,
            "overflow-y-auto text-xs py-2 flex flex-col",
            html! {
                div
                    class="h-full"
                    hx-get=(format!("/api/widgets/{}?group={}&title={}", self.name(), group_id, service.title))
                    hx-trigger="load"
                    hx-swap="innerHTML" {
                        div class="flex items-center justify-center h-full" {
                            div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500" {}
                        }
                    }
            },
        )
    }

    async fn handle_request(
        &self,
        state: Arc<AppState>,
        query: WidgetQuery,
    ) -> VestaResult<Markup> {
        let config = state.get_config()?;
        let widget_info = config
            .get_widget(&query.group, &query.title)
            .ok_or_else(|| VestaError::WidgetNotFound {
                group: query.group.clone(),
                title: query.title.clone(),
            })?;

        let (url, key) = self.get_widget_credentials(widget_info)?;

        let client = state.get_http_client();
        let mut calendar = self.fetch_calendar(client, url, key).await?;
        let download_queue = self.fetch_download_queue(client, url, key).await?;

        let download_queue_ids: HashSet<u32> = download_queue
            .records
            .into_iter()
            .map(|record| record.series_id)
            .collect();

        for entry in &mut calendar {
            entry.is_downloading = download_queue_ids.contains(&entry.series_id);
        }

        let current_date = Utc::now();
        let calendar_grouped: IndexMap<String, Vec<&CalendarEntry>> =
            calendar.iter().fold(IndexMap::new(), |mut acc, entry| {
                let formatted_date = entry
                    .air_date_utc
                    .with_timezone(&Local)
                    .format("%Y-%m-%d")
                    .to_string();
                acc.entry(formatted_date).or_default().push(entry);
                acc
            });

        Ok(html! {
            @if calendar_grouped.is_empty() {
                div class="flex justify-center items-center min-w-full min-h-full text-xl font-bold" {
                    "No entry"
                }
            } @else {
                @for (date, entries) in calendar_grouped {
                    div class="flex justify-center py-2 my-2 min-w-full rounded bg-sky-400" {
                        a href=(format!("{}/calendar", url)) class="font-semibold text-center" {
                            (date)
                        }
                    }
                    @for entry in entries {
                        div."px-2"."my-2".(self.get_entry_class(entry, &current_date)) {
                            a href=(self.format_series_url(url, &entry.series.title_slug)) class="line-clamp-1 hover:brightness-125" {
                                (entry.series.title)
                            }
                            span class="block text-xs text-slate-400" {
                                (self.format_episode(entry))
                            }
                            span class="text-xs text-slate-500" {
                                (self.format_air_time(entry))
                            }
                        }
                    }
                }
            }
        })
    }

    fn validate_config(&self, widget: &Widget) -> VestaResult<()> {
        let config = widget
            .config
            .as_ref()
            .ok_or_else(|| VestaError::MissingCredentials {
                field: "config".to_string(),
            })?;

        if !config.contains_key("url") {
            return Err(VestaError::MissingCredentials {
                field: "url".to_string(),
            });
        }

        if !config.contains_key("key") {
            return Err(VestaError::MissingCredentials {
                field: "key".to_string(),
            });
        }

        Ok(())
    }
}
