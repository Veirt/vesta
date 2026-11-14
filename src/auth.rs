use std::time::{Duration, SystemTime, UNIX_EPOCH};

use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::Json;
use axum::body::Body;
use axum::extract::Extension;
use axum::http::header::{COOKIE, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue, Request, Response, StatusCode};
use axum::middleware::Next;
use axum::response::IntoResponse;
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::AppState;
use crate::error::{VestaError, VestaResult};
use crate::response::jsend;

const SESSION_COOKIE: &str = "vesta_session";
const SESSION_LIFETIME: Duration = Duration::from_secs(60 * 60 * 24 * 7);
const MIN_PASSWORD_LENGTH: usize = 12;

#[derive(Deserialize)]
pub struct PasswordRequest {
    password: String,
}

pub async fn status(
    Extension(state): Extension<std::sync::Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, VestaError> {
    let setup_required = state.database.admin_password_hash().await?.is_none();
    let authenticated = match (setup_required, session_token(&headers)) {
        (false, Some(token)) => state.database.session_is_valid(&token_hash(&token)).await?,
        _ => false,
    };
    Ok(jsend::success(json!({
        "setup_required": setup_required,
        "authenticated": authenticated,
    })))
}

pub async fn setup(
    Extension(state): Extension<std::sync::Arc<AppState>>,
    Json(request): Json<PasswordRequest>,
) -> VestaResult<Response<Body>> {
    validate_password(&request.password)?;
    let password = request.password;
    let password_hash = tokio::task::spawn_blocking(move || {
        Argon2::default()
            .hash_password(password.as_bytes())
            .map(|hash| hash.to_string())
            .map_err(|error| VestaError::Config(format!("password hashing failed: {error}")))
    })
    .await
    .map_err(|error| VestaError::Config(format!("password hashing task failed: {error}")))??;

    if !state.database.create_admin(&password_hash).await? {
        return Err(VestaError::ApiError {
            status: StatusCode::CONFLICT,
            message: "Administrator setup has already been completed".to_string(),
        });
    }

    create_session_response(&state, "Administrator account created").await
}

pub async fn login(
    Extension(state): Extension<std::sync::Arc<AppState>>,
    Json(request): Json<PasswordRequest>,
) -> VestaResult<Response<Body>> {
    let Some(password_hash) = state.database.admin_password_hash().await? else {
        return Err(VestaError::ApiError {
            status: StatusCode::CONFLICT,
            message: "Administrator setup is required before login".to_string(),
        });
    };

    let password = request.password;
    let verified = tokio::task::spawn_blocking(move || {
        let hash = PasswordHash::new(&password_hash).map_err(|error| {
            VestaError::Config(format!("stored password hash is invalid: {error}"))
        })?;
        Ok::<_, VestaError>(
            Argon2::default()
                .verify_password(password.as_bytes(), &hash)
                .is_ok(),
        )
    })
    .await
    .map_err(|error| VestaError::Config(format!("password verification task failed: {error}")))??;

    if !verified {
        return Err(VestaError::ApiError {
            status: StatusCode::UNAUTHORIZED,
            message: "Invalid password".to_string(),
        });
    }

    create_session_response(&state, "Signed in").await
}

pub async fn logout(
    Extension(state): Extension<std::sync::Arc<AppState>>,
    headers: HeaderMap,
) -> VestaResult<Response<Body>> {
    if let Some(token) = session_token(&headers) {
        state.database.delete_session(&token_hash(&token)).await?;
    }

    let mut response = jsend::success(json!({ "message": "Signed out" })).into_response();
    response.headers_mut().insert(
        SET_COOKIE,
        HeaderValue::from_static("vesta_session=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0"),
    );
    Ok(response)
}

pub async fn require_admin(
    Extension(state): Extension<std::sync::Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Response<Body> {
    let token = request
        .headers()
        .get(COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(parse_session_cookie);
    let authorized = match token {
        Some(token) => match state.database.session_is_valid(&token_hash(token)).await {
            Ok(valid) => valid,
            Err(error) => return error.into_response(),
        },
        None => false,
    };

    if authorized {
        next.run(request).await
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "status": "error",
                "error": "authentication_required",
                "message": "Sign in to manage dashboard settings",
            })),
        )
            .into_response()
    }
}

fn validate_password(password: &str) -> VestaResult<()> {
    let length = password.chars().count();
    if !(MIN_PASSWORD_LENGTH..=256).contains(&length) {
        return Err(VestaError::ApiError {
            status: StatusCode::BAD_REQUEST,
            message: format!("Password must contain {MIN_PASSWORD_LENGTH} to 256 characters"),
        });
    }
    Ok(())
}

async fn create_session_response(state: &AppState, message: &str) -> VestaResult<Response<Body>> {
    let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let expires_at = SystemTime::now()
        .checked_add(SESSION_LIFETIME)
        .and_then(|expiry| expiry.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs() as i64)
        .ok_or_else(|| VestaError::Config("session expiry is out of range".to_string()))?;
    state
        .database
        .store_session(&token_hash(&token), expires_at)
        .await?;

    let secure = if state.cookie_secure { "; Secure" } else { "" };
    let cookie = format!(
        "{SESSION_COOKIE}={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}{secure}",
        SESSION_LIFETIME.as_secs()
    );
    let cookie = HeaderValue::from_str(&cookie)
        .map_err(|error| VestaError::Config(format!("failed to create session cookie: {error}")))?;
    let mut response = jsend::success(json!({ "message": message })).into_response();
    response.headers_mut().insert(SET_COOKIE, cookie);
    Ok(response)
}

fn session_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get(COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(parse_session_cookie)
        .map(str::to_string)
}

fn parse_session_cookie(cookie: &str) -> Option<&str> {
    cookie.split(';').find_map(|pair| {
        let (name, value) = pair.trim().split_once('=')?;
        (name == SESSION_COOKIE && !value.is_empty()).then_some(value)
    })
}

fn token_hash(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::{parse_session_cookie, token_hash, validate_password};

    #[test]
    fn parses_only_the_vesta_session_cookie() {
        assert_eq!(
            parse_session_cookie("theme=dark; vesta_session=abc123; other=x"),
            Some("abc123")
        );
        assert_eq!(parse_session_cookie("session=abc123"), None);
    }

    #[test]
    fn hashes_tokens_without_preserving_the_cookie_secret() {
        assert_ne!(token_hash("opaque-session-token"), "opaque-session-token");
        assert_eq!(token_hash("same"), token_hash("same"));
    }

    #[test]
    fn first_run_password_requires_minimum_length() {
        assert!(validate_password("short").is_err());
        assert!(validate_password("a sufficiently long passphrase").is_ok());
    }
}
