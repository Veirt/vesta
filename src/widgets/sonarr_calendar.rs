use std::{collections::HashSet, sync::Arc};

use axum::{extract::Query, response::IntoResponse, Extension, Json};
use chrono::{DateTime, Duration, Local, Utc};
use indexmap::IndexMap;
use maud::{html, Markup};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;

use crate::{
    config::{Service, Widget},
    AppState,
};

#[derive(Error, Debug)]
enum SonarrError {
    #[error("Failed to send request: {0}")]
    RequestFailed(#[from] reqwest::Error),
    #[error("API returned non-success status code: {0}")]
    ApiError(StatusCode),
    #[error("Missing URL or API key")]
    MissingCredentials,
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

async fn fetch_download_queue(url: &str, key: &str) -> Result<DownloadQueue, SonarrError> {
    let client = Client::new();
    let response = client
        .get(format!("{}/api/v3/queue", url))
        .header("X-Api-Key", key)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(SonarrError::ApiError(response.status()));
    }

    let download_queue = response.json::<DownloadQueue>().await?;
    Ok(download_queue)
}

async fn fetch_calendar(url: &str, key: &str) -> Result<Calendar, SonarrError> {
    let today = Utc::now();
    let day_after_tomorrow = today + Duration::days(2);
    let params = [
        ("unmonitored", "false"),
        ("includeSeries", "true"),
        ("start", &today.format("%Y-%m-%d").to_string()),
        ("end", &day_after_tomorrow.format("%Y-%m-%d").to_string()),
    ];

    let client = Client::new();
    let response = client
        .get(format!("{}/api/v3/calendar", url))
        .query(&params)
        .header("X-Api-Key", key)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(SonarrError::ApiError(response.status()));
    }

    let calendar = response.json::<Calendar>().await?;
    Ok(calendar)
}

fn get_widget_credentials(widget_info: &Widget) -> Result<(&str, &str), SonarrError> {
    widget_info
        .config
        .as_ref()
        .and_then(|config| {
            let url = config.get("url")?;
            let key = config.get("key")?;
            Some((url.as_str(), key.as_str()))
        })
        .ok_or_else(|| SonarrError::MissingCredentials)
}

fn format_time(date: &DateTime<Utc>) -> String {
    date.with_timezone(&Local).format("%H:%M").to_string()
}

fn add_minutes(date: &DateTime<Utc>, minutes: i64) -> DateTime<Utc> {
    *date + Duration::minutes(minutes)
}

fn format_air_time(entry: &CalendarEntry) -> String {
    let air_date = format_time(&entry.air_date_utc);
    let aired_date = format_time(&add_minutes(&entry.air_date_utc, entry.series.runtime));
    format!("{} - {}", air_date, aired_date)
}

fn format_episode(entry: &CalendarEntry) -> String {
    format!("{}x{:02}", entry.season_number, entry.episode_number)
}

fn format_series_url(sonarr_url: &str, title_slug: &str) -> String {
    format!("{}/series/{}", sonarr_url, title_slug)
}

fn get_entry_class(entry: &CalendarEntry, current_date: &DateTime<Utc>) -> &'static str {
    let aired_date = add_minutes(&entry.air_date_utc, entry.series.runtime);

    if *current_date < entry.air_date_utc {
        "unaired"
    } else if entry.is_downloading {
        "downloading"
    } else if entry.has_file {
        "downloaded"
    } else if *current_date >= entry.air_date_utc && *current_date <= aired_date {
        "onAir"
    } else if !entry.has_file && *current_date > aired_date {
        "missing"
    } else {
        ""
    }
}

#[derive(Deserialize)]
pub struct QueryParams {
    group: String,
    title: String,
}
pub async fn sonarr_calendar_handler(
    Extension(state): Extension<Arc<AppState>>,
    Query(params): Query<QueryParams>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let config = &state.get_config();
    let widget_info = config
        .get_widget(&params.group, &params.title)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({"status": "fail", "message": "Widget info not found"})),
            )
        })?;

    let (url, key) = get_widget_credentials(widget_info).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"status": "fail", "message": e.to_string(), "data": format!("{:?}", e)})),
        )
    })?;

    let mut calendar = fetch_calendar(url, key).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"status": "error", "message": e.to_string(), "data": format!("{:?}", e)})),
        )
    })?;

    let download_queue = fetch_download_queue(url, key).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"status": "error", "message": e.to_string(), "data": format!("{:?}", e)})),
        )
    })?;

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
                div class="flex justify-center py-2 my-2 min-w-full rounded bg-accent" {
                    a href=(format!("{}/calendar", url)) class="font-semibold text-center" {
                        (date)
                    }
                }
                @for entry in entries {
                    div."px-2"."my-2".(get_entry_class(entry, &current_date)) {
                        a href=(format_series_url(url, &entry.series.title_slug)) class="line-clamp-1 hover:brightness-125" {
                            (entry.series.title)
                        }
                        span class="block text-xs text-slate-400" {
                            (format_episode(entry))
                        }
                        span class="text-xs text-slate-500" {
                            (format_air_time(entry))
                        }
                    }
                }
            }
        }
    })
}

pub fn render_sonarr_calendar_widget(group_id: &str, service_info: &Service) -> Markup {
    let width = service_info.width.unwrap_or(1);
    let height = service_info.height.unwrap_or(1);

    html! {
        div class="overflow-y-auto no-scrollbar text-xs bg-black-2 rounded-xl py-2 m-2 flex flex-col" data-width=(width) data-height=(height) {
            div class="overflow-y-auto no-scrollbar" {
                div
                    hx-get=(format!("/api/sonarr-calendar?group={}&title={}", group_id, service_info.title))
                    hx-trigger="load"
                    hx-swap="innerHTML"
                { }
            }
        }
    }
}
