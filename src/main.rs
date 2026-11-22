mod api;
mod auth;
mod config;
mod config_codec;
mod config_layout;
mod config_manager;
mod database;
mod error;
mod http_client;
mod icons_api;
mod metrics;
mod plugins;
mod plugins_api;
mod response;
mod router;
mod services;
mod settings_api;
mod state;
mod templates;
mod validation;
mod widget_system;
mod widgets;

use tracing_subscriber::EnvFilter;

pub(crate) use state::AppState;

pub(crate) struct Settings {
    pub(crate) config_path: String,
    pub(crate) database_path: String,
    pub(crate) plugins_dir: String,
    pub(crate) bind_addr: String,
    pub(crate) static_dir: String,
    pub(crate) cookie_secure: bool,
}

impl Settings {
    fn from_env() -> Self {
        Self {
            config_path: env_or("VESTA_CONFIG", "./config.toml"),
            database_path: env_or("VESTA_DATABASE", "./data/vesta.sqlite"),
            plugins_dir: env_or("VESTA_PLUGINS", "./data/plugins"),
            bind_addr: env_or("VESTA_BIND", "0.0.0.0:3000"),
            static_dir: env_or("VESTA_STATIC_DIR", "static"),
            cookie_secure: env_or("VESTA_COOKIE_SECURE", "false") == "true",
        }
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

async fn shutdown() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut term = signal(SignalKind::terminate()).expect("install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = term.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await.ok();
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "vesta=info".into()))
        .init();

    let settings = Settings::from_env();
    let state = match AppState::new(&settings).await {
        Ok(state) => state,
        Err(e) => {
            tracing::error!(error = %e, "startup failed");
            std::process::exit(1);
        }
    };

    let app = router::build_router(state.clone(), &settings.static_dir);
    let listener = tokio::net::TcpListener::bind(&settings.bind_addr)
        .await
        .expect("bind listen address");

    tracing::info!(addr = %settings.bind_addr, "vesta listening");
    tracing::debug!(
        database = %settings.database_path,
        widgets = ?state.widget_registry.current().names(),
        "registered widgets"
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown())
        .await
        .expect("server run");

    state.nodes.shutdown();
    tracing::info!("vesta stopped");
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use http_client::create_default_client;
    use widget_system::WidgetRegistry;

    #[test]
    fn builtin_registry_exposes_expected_sizes_and_catalog_visibility() {
        let builtins = AppState::builtin_widgets(create_default_client().unwrap());
        let mut registry = WidgetRegistry::new();
        for handler in &builtins {
            registry = registry.register_shared(Arc::clone(handler));
        }

        for (name, expected) in [
            ("Clock", (4, 2)),
            ("QuickLinks", (3, 2)),
            ("SystemStats", (6, 4)),
            ("Weather", (4, 3)),
            ("SonarrCalendar", (4, 4)),
            ("Prowlarr", (4, 2)),
            ("AdGuard", (4, 2)),
            ("JellyfinRandomShow", (4, 3)),
        ] {
            assert_eq!(registry.default_size(name), Some(expected), "{name}");
        }

        assert!(registry.get("SystemStats").is_some());
        assert!(!registry.get("SystemStats").unwrap().catalog_visible());
        assert!(
            !registry
                .catalog_descriptors()
                .iter()
                .any(|d| d.name == "SystemStats")
        );
    }

    #[tokio::test]
    async fn http_router_protects_admin_routes_and_completes_plugin_lifecycle() {
        let (_sandbox, settings, state) = state::test_app_state("vesta-http-router").await;
        let router = router::build_router(state.clone(), &settings.static_dir);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });

        let base = format!("http://{addr}");
        let client = reqwest::Client::new();

        assert_eq!(
            client
                .get(format!("{base}/api/health"))
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::OK
        );
        assert_eq!(
            client
                .get(format!("{base}/api/settings"))
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            client
                .get(format!("{base}/api/plugins"))
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::UNAUTHORIZED
        );

        let setup_res = client
            .post(format!("{base}/api/auth/setup"))
            .json(&serde_json::json!({ "password": "correct-horse-battery" }))
            .send()
            .await
            .unwrap();
        assert_eq!(setup_res.status(), reqwest::StatusCode::OK);
        let cookie_header = setup_res
            .headers()
            .get(reqwest::header::SET_COOKIE)
            .expect("setup sets session cookie")
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_string();

        let settings_before: serde_json::Value = client
            .get(format!("{base}/api/settings"))
            .header(reqwest::header::COOKIE, &cookie_header)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let names_before: Vec<_> = settings_before["data"]["widgets"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|d| d["name"].as_str())
            .collect();
        assert!(!names_before.contains(&"SystemStats"));

        let repo = crate::plugins::test_fixtures::git_fixture_repo(
            &crate::plugins::test_fixtures::fixture_dir("hello-world"),
            "http-lifecycle",
        );
        let install_res = client
            .post(format!("{base}/api/plugins"))
            .header(reqwest::header::COOKIE, &cookie_header)
            .json(&serde_json::json!({ "url": repo.path().display().to_string() }))
            .send()
            .await
            .unwrap();
        assert_eq!(install_res.status(), reqwest::StatusCode::OK);
        let install_body: serde_json::Value = install_res.json().await.unwrap();
        assert_eq!(install_body["data"]["id"], "hello-world");

        let update_res = client
            .put(format!("{base}/api/plugins/hello-world"))
            .header(reqwest::header::COOKIE, &cookie_header)
            .send()
            .await
            .unwrap();
        assert_eq!(update_res.status(), reqwest::StatusCode::OK);

        let asset_res = client
            .get(format!("{base}/api/plugins/hello-world/assets/index.html"))
            .send()
            .await
            .unwrap();
        assert_eq!(asset_res.status(), reqwest::StatusCode::OK);
        assert_eq!(
            asset_res
                .headers()
                .get("x-content-type-options")
                .and_then(|v| v.to_str().ok()),
            Some("nosniff")
        );
        assert_eq!(
            asset_res
                .headers()
                .get("cache-control")
                .and_then(|v| v.to_str().ok()),
            Some("no-cache")
        );
        assert_eq!(
            asset_res
                .headers()
                .get("content-security-policy")
                .and_then(|v| v.to_str().ok()),
            Some("frame-ancestors 'self'")
        );
        assert!(
            asset_res
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .starts_with("text/html")
        );
        assert_eq!(
            client
                .get(format!(
                    "{base}/api/plugins/hello-world/assets/..%2fvesta-plugin.toml"
                ))
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::NOT_FOUND
        );

        let save_tile_res = client
            .put(format!("{base}/api/settings/dashboard"))
            .header(reqwest::header::COOKIE, &cookie_header)
            .json(&serde_json::json!({
                "items": [{
                    "kind": "widget",
                    "id": "counter-1",
                    "title": "Counter",
                    "name": "Plugin:hello-world:counter",
                    "config": { "label": "Hits", "start_value": 2 },
                    "x": 0,
                    "y": 0,
                    "width": 3,
                    "height": 2
                }]
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(save_tile_res.status(), reqwest::StatusCode::OK);

        let widget_res: serde_json::Value = client
            .get(format!(
                "{base}/api/widgets/Plugin:hello-world:counter?id=counter-1"
            ))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(widget_res["data"]["widget"]["plugin_id"], "hello-world");
        assert_eq!(widget_res["data"]["widget"]["widget_id"], "counter");
        assert_eq!(widget_res["data"]["widget"]["config"]["label"], "Hits");

        let blocked_delete = client
            .delete(format!("{base}/api/plugins/hello-world"))
            .header(reqwest::header::COOKIE, &cookie_header)
            .send()
            .await
            .unwrap();
        assert_eq!(blocked_delete.status(), reqwest::StatusCode::BAD_REQUEST);

        assert_eq!(
            client
                .put(format!("{base}/api/settings/dashboard"))
                .header(reqwest::header::COOKIE, &cookie_header)
                .json(&serde_json::json!({ "items": [] }))
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::OK
        );
        let uninstall_res = client
            .delete(format!("{base}/api/plugins/hello-world"))
            .header(reqwest::header::COOKIE, &cookie_header)
            .send()
            .await
            .unwrap();
        assert_eq!(uninstall_res.status(), reqwest::StatusCode::OK);
        let uninstall_body: serde_json::Value = uninstall_res.json().await.unwrap();
        assert_eq!(uninstall_body["data"]["removed"], "hello-world");

        assert_eq!(
            client
                .get(format!("{base}/api/dashboard"))
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::OK
        );
        assert_eq!(
            client
                .get(format!("{base}/api/dashboard/services"))
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::OK
        );

        state.nodes.shutdown();
        server.abort();
    }
}
