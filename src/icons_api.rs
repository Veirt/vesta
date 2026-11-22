use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Extension, Query};
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use serde::Deserialize;
use serde_json::json;

use crate::AppState;
use crate::error::{VestaError, VestaResult};
use crate::response::jsend;

#[derive(Deserialize)]
pub struct IconUploadQuery {
    name: String,
}

pub async fn upload_icon(
    Extension(state): Extension<Arc<AppState>>,
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
