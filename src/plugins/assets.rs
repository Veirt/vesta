use std::path::{Path, PathBuf};

/// Resolve a request path against a plugin directory, refusing anything that
/// escapes it. Request paths arrive percent-decoded by axum, so encoded
/// traversal (`%2e%2e`) is caught here too.
pub fn resolve_asset_path(plugin_root: &Path, requested: &str) -> Option<PathBuf> {
    if requested.is_empty() {
        return None;
    }
    let relative = Path::new(requested);
    if relative.is_absolute() {
        return None;
    }
    if !relative.components().all(|component| {
        matches!(
            component,
            std::path::Component::Normal(_) | std::path::Component::CurDir
        )
    }) {
        return None;
    }
    let resolved = plugin_root.join(relative);
    if !resolved.is_file() {
        return None;
    }
    // Final containment check: symlinks inside a cloned repository could
    // otherwise point outside the plugin directory.
    let canonical = resolved.canonicalize().ok()?;
    let canonical_root = plugin_root.canonicalize().ok()?;
    if canonical.starts_with(&canonical_root) {
        Some(canonical)
    } else {
        None
    }
}

/// Content type for a served file, by extension. Unknown extensions are
/// rejected instead of guessed: plugin pages only need the common web types.
pub fn content_type(path: &Path) -> Option<&'static str> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    Some(match extension.as_str() {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" | "map" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "ico" => "image/x-icon",
        "txt" => "text/plain; charset=utf-8",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "wasm" => "application/wasm",
        "xml" => "application/xml",
        "webmanifest" => "application/manifest+json",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plugin_dir() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "vesta-assets-test-{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(root.join("assets")).unwrap();
        std::fs::write(root.join("index.html"), "<html></html>").unwrap();
        std::fs::write(root.join("assets").join("style.css"), "body{}").unwrap();
        root
    }

    #[test]
    fn resolves_files_inside_the_plugin_directory() {
        let root = plugin_dir();
        assert_eq!(
            resolve_asset_path(&root, "index.html"),
            Some(root.join("index.html").canonicalize().unwrap())
        );
        assert_eq!(
            resolve_asset_path(&root, "assets/style.css"),
            Some(
                root.join("assets")
                    .join("style.css")
                    .canonicalize()
                    .unwrap()
            )
        );
        // CurDir components are tolerated and normalized.
        assert!(resolve_asset_path(&root, "./index.html").is_some());
    }

    #[test]
    fn rejects_traversal_and_missing_files() {
        let root = plugin_dir();
        for bad in [
            "../secret.txt",
            "..%2fsecret.txt",
            "assets/../../../etc/passwd",
            "/etc/passwd",
            "",
            "missing.html",
            "assets",
        ] {
            assert!(
                resolve_asset_path(&root, bad).is_none(),
                "expected rejection for '{bad}'"
            );
        }
    }

    #[test]
    fn rejects_symlink_escape() {
        let root = plugin_dir();
        let outside = std::env::temp_dir().join("vesta-assets-outside.html");
        std::fs::write(&outside, "secret").unwrap();
        std::os::unix::fs::symlink(&outside, root.join("leak.html")).unwrap();
        assert!(resolve_asset_path(&root, "leak.html").is_none());
        std::fs::remove_file(&outside).ok();
    }

    #[test]
    fn content_types_cover_common_web_files() {
        assert_eq!(
            content_type(Path::new("index.html")),
            Some("text/html; charset=utf-8")
        );
        assert_eq!(
            content_type(Path::new("a.JS")),
            Some("text/javascript; charset=utf-8")
        );
        assert_eq!(content_type(Path::new("banner.svg")), Some("image/svg+xml"));
        assert_eq!(content_type(Path::new("plugin.exe")), None);
        assert_eq!(content_type(Path::new("noext")), None);
    }
}
