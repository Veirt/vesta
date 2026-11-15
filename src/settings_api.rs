use axum::Json;
use axum::body::Bytes;
use axum::extract::{Extension, Path, Query};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::json;

use crate::AppState;
use crate::config::{AppSettings, Dashboard, DashboardGroup, DashboardItem};
use crate::error::{VestaError, VestaResult};
use crate::metrics::node::NodeConfig;
use crate::response::jsend;

#[derive(Deserialize)]
pub struct DashboardInput {
    #[serde(default)]
    nodes: Vec<NodeConfig>,
}

#[derive(Deserialize)]
pub struct SettingsInput {
    dashboard: DashboardInput,
    #[serde(default)]
    app: AppSettings,
}

#[derive(Deserialize)]
pub struct IconUploadQuery {
    name: String,
}

#[derive(Deserialize)]
pub struct PingTestInput {
    url: String,
}

#[derive(Deserialize)]
pub struct DashboardItemsInput {
    #[serde(default)]
    items: Vec<DashboardItem>,
    groups: Option<Vec<DashboardGroup>>,
}

pub async fn get_settings(
    Extension(state): Extension<std::sync::Arc<AppState>>,
) -> Result<impl IntoResponse, VestaError> {
    let snapshot = state.config.settings_snapshot();
    Ok(jsend::success(json!({
        "dashboard": {
            "nodes": snapshot.dashboard.nodes,
            "items": snapshot.dashboard.items,
            "groups": snapshot.dashboard.groups,
        },
        "app": snapshot.app,
        "widgets": state.widget_registry.descriptors().into_iter()
            .filter(|widget| widget.name != "SystemStats")
            .collect::<Vec<_>>(),
    })))
}

pub async fn save_settings(
    Extension(state): Extension<std::sync::Arc<AppState>>,
    Json(input): Json<SettingsInput>,
) -> VestaResult<impl IntoResponse> {
    let snapshot = state.config.settings_snapshot();
    let dashboard = Dashboard {
        nodes: input.dashboard.nodes,
        items: snapshot.dashboard.items.clone(),
        groups: snapshot.dashboard.groups.clone(),
    };

    state.config.save(dashboard.clone(), input.app).await?;
    state.nodes.reconfigure(&dashboard.nodes)?;
    Ok(jsend::success_message("Dashboard settings saved"))
}

pub async fn save_dashboard_items(
    Extension(state): Extension<std::sync::Arc<AppState>>,
    Json(input): Json<DashboardItemsInput>,
) -> VestaResult<impl IntoResponse> {
    let snapshot = state.config.settings_snapshot();
    let dashboard = Dashboard {
        nodes: snapshot.dashboard.nodes.clone(),
        items: input.items,
        groups: input
            .groups
            .unwrap_or_else(|| snapshot.dashboard.groups.clone()),
    };
    state.config.save(dashboard, snapshot.app.clone()).await?;
    Ok(jsend::success_message("Dashboard layout saved"))
}

pub async fn test_ping_url(
    Extension(state): Extension<std::sync::Arc<AppState>>,
    Json(input): Json<PingTestInput>,
) -> VestaResult<impl IntoResponse> {
    let valid = reqwest::Url::parse(&input.url)
        .ok()
        .is_some_and(|url| matches!(url.scheme(), "http" | "https") && url.host_str().is_some());
    if !valid {
        return Err(VestaError::InvalidDashboard(
            "availability check must be a valid http(s) URL".to_string(),
        ));
    }
    let up = state.ping.test_url(&input.url).await;
    Ok(jsend::success(json!({ "up": up })))
}

pub async fn upload_icon(
    Extension(state): Extension<std::sync::Arc<AppState>>,
    Query(query): Query<IconUploadQuery>,
    headers: HeaderMap,
    bytes: Bytes,
) -> VestaResult<impl IntoResponse> {
    if bytes.is_empty() || bytes.len() > 2 * 1024 * 1024 {
        return Err(VestaError::ApiError {
            status: StatusCode::PAYLOAD_TOO_LARGE,
            message: "Icon must be between 1 byte and 2 MiB".to_string(),
        });
    }
    let content_type = headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(str::trim)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !valid_icon_data(&content_type, &bytes) {
        return Err(VestaError::ApiError {
            status: StatusCode::UNSUPPORTED_MEDIA_TYPE,
            message: "Upload a valid PNG, JPEG, WebP, or GIF icon".to_string(),
        });
    }

    let name = sanitize_icon_name(&query.name);
    let id = state
        .database
        .store_icon(&name, &content_type, &bytes)
        .await?;
    Ok(jsend::success(json!({
        "id": id,
        "name": name,
        "content_type": content_type,
        "url": format!("/api/icons/{id}"),
    })))
}

pub async fn get_icon(
    Extension(state): Extension<std::sync::Arc<AppState>>,
    Path(icon_id): Path<i64>,
) -> VestaResult<Response> {
    let Some(icon) = state.database.load_icon(icon_id).await? else {
        return Err(VestaError::ApiError {
            status: StatusCode::NOT_FOUND,
            message: "Icon not found".to_string(),
        });
    };
    let mut response = (StatusCode::OK, icon.data).into_response();
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_str(&icon.content_type).map_err(|error| {
            VestaError::Config(format!("invalid stored icon content type: {error}"))
        })?,
    );
    response.headers_mut().insert(
        CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=300"),
    );
    response.headers_mut().insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    Ok(response)
}

fn valid_icon_data(content_type: &str, bytes: &[u8]) -> bool {
    match content_type {
        "image/png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "image/jpeg" => bytes.starts_with(&[0xff, 0xd8, 0xff]),
        "image/webp" => bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP",
        "image/gif" => bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a"),
        _ => false,
    }
}

fn sanitize_icon_name(name: &str) -> String {
    let basename = name.rsplit(['/', '\\']).next().unwrap_or("icon").trim();
    let cleaned: String = basename
        .chars()
        .filter(|character| !character.is_control())
        .take(120)
        .collect();
    if cleaned.is_empty() {
        "icon".to_string()
    } else {
        cleaned
    }
}

#[cfg(test)]
mod tests {
    use super::{sanitize_icon_name, valid_icon_data};

    #[test]
    fn uploaded_icons_must_match_a_supported_raster_signature() {
        assert!(valid_icon_data("image/png", b"\x89PNG\r\n\x1a\nrest"));
        assert!(valid_icon_data("image/webp", b"RIFF0000WEBPrest"));
        assert!(!valid_icon_data("image/png", b"<svg></svg>"));
        assert!(!valid_icon_data("image/svg+xml", b"<svg></svg>"));
    }

    #[test]
    fn icon_filename_cannot_include_a_path() {
        assert_eq!(sanitize_icon_name("../../jellyfin.png"), "jellyfin.png");
        assert_eq!(sanitize_icon_name("\n"), "icon");
    }
}
