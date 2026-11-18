/// Identifier, link, and URL validation primitives shared across the crate.
pub fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

pub fn valid_http_url(value: &str) -> bool {
    reqwest::Url::parse(value)
        .ok()
        .is_some_and(|url| matches!(url.scheme(), "http" | "https") && url.host_str().is_some())
}

pub fn valid_link(value: &str) -> bool {
    if value.starts_with('/') && !value.starts_with("//") {
        return true;
    }
    valid_http_url(value)
}

pub fn valid_image_url(value: &str) -> bool {
    if value.starts_with("/api/icons/") {
        return value
            .strip_prefix("/api/icons/")
            .is_some_and(|id| !id.is_empty() && id.bytes().all(|byte| byte.is_ascii_digit()));
    }
    valid_http_url(value)
}
