use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use chrono::Utc;
use tokio::sync::Mutex;

use crate::error::{VestaError, VestaResult};
use crate::plugins::manifest::{
    PluginRecord, PluginSource, SOURCE_FILE, is_valid_slug, load_manifest,
};

/// Timeout for the whole clone+validate step. The blocking worker polls the
/// shared interrupt flag, which is set when the timeout elapses, and its
/// temporary directory is removed afterwards.
const CLONE_TIMEOUT: Duration = Duration::from_secs(120);

pub struct PluginManager {
    root: PathBuf,
    /// Serialize installs and removals so two simultaneous installs of the
    /// same plugin id cannot race past the duplicate check.
    gate: Mutex<()>,
}

fn plugin_error(message: impl Into<String>) -> VestaError {
    VestaError::Plugin(message.into())
}

fn plugin_not_found(id: &str) -> VestaError {
    VestaError::PluginNotFound { id: id.to_string() }
}

impl PluginManager {
    pub fn new(root: impl Into<PathBuf>) -> VestaResult<Self> {
        let root = root.into();
        std::fs::create_dir_all(&root)?;
        Ok(Self {
            root,
            gate: Mutex::new(()),
        })
    }

    #[cfg(test)]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// All currently installed plugins, ordered by id. Unreadable or invalid
    /// directories are skipped with a warning: one broken plugin must not take
    /// down the widget registry.
    pub fn load_all(&self) -> Vec<PluginRecord> {
        let mut ids: Vec<String> = std::fs::read_dir(&self.root)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .filter(|entry| entry.file_type().map(|t| t.is_dir()).unwrap_or(false))
                    .filter_map(|entry| entry.file_name().into_string().ok())
                    .filter(|name| !name.starts_with('.'))
                    .filter(|name| is_valid_slug(name))
                    .collect()
            })
            .unwrap_or_default();
        ids.sort();
        ids.into_iter()
            .filter_map(|id| match self.record(&id) {
                Ok(Some(record)) => Some(record),
                Ok(None) => None,
                Err(error) => {
                    tracing::warn!(plugin = %id, %error, "skipping invalid plugin directory");
                    None
                }
            })
            .collect()
    }

    /// Load one plugin record from its directory. `Ok(None)` means the
    /// directory does not exist.
    pub fn record(&self, id: &str) -> VestaResult<Option<PluginRecord>> {
        if !is_valid_slug(id) {
            return Ok(None);
        }
        let dir = self.plugin_dir(id);
        if !dir.is_dir() {
            return Ok(None);
        }
        let manifest = load_manifest(&dir)?;
        let source = PluginSource::from_toml(&std::fs::read_to_string(dir.join(SOURCE_FILE))?)?;
        Ok(Some(PluginRecord { manifest, source }))
    }

    pub fn plugin_dir(&self, id: &str) -> PathBuf {
        self.root.join(id)
    }

    /// Shallow-clone `url`, validate its manifest, strip `.git`, then move the
    /// tree into place under the manifest's plugin id. The clone runs on a
    /// blocking worker with a shared interrupt flag; on timeout the worker is
    /// told to stop and its temporary directory cleaned up once it exits.
    pub async fn install(&self, url: &str) -> VestaResult<PluginRecord> {
        if url.trim().is_empty() {
            return Err(plugin_error("a git repository URL is required"));
        }
        let _gate = self.gate.lock().await;
        let temp_dir = self
            .root
            .join(format!(".tmp-{}", uuid::Uuid::new_v4().simple()));
        let interrupt = Arc::new(AtomicBool::new(false));
        let worker = {
            let temp_dir = temp_dir.clone();
            let interrupt = Arc::clone(&interrupt);
            let url = url.to_string();
            tokio::task::spawn_blocking(move || clone_plugin(&url, &temp_dir, &interrupt))
        };
        let mut pinned = Box::pin(worker);

        match tokio::time::timeout(CLONE_TIMEOUT, pinned.as_mut()).await {
            Ok(Ok(Ok(record))) => {
                std::fs::remove_dir_all(temp_dir.join(".git")).ok();
                let destination = self.plugin_dir(&record.manifest.id);
                if destination.exists() {
                    std::fs::remove_dir_all(&temp_dir).ok();
                    return Err(plugin_error(format!(
                        "plugin '{}' is already installed: remove it before installing again",
                        record.manifest.id
                    )));
                }
                std::fs::rename(&temp_dir, &destination)?;
                Ok(record)
            }
            Ok(Ok(Err(message))) => {
                std::fs::remove_dir_all(&temp_dir).ok();
                Err(plugin_error(message))
            }
            Ok(Err(join)) => {
                std::fs::remove_dir_all(&temp_dir).ok();
                Err(plugin_error(format!("clone worker failed: {join}")))
            }
            Err(_elapsed) => {
                interrupt.store(true, Ordering::Relaxed);
                let cleanup_dir = temp_dir.clone();
                tokio::spawn(async move {
                    let _ = pinned.await;
                    std::fs::remove_dir_all(&cleanup_dir).ok();
                });
                Err(plugin_error(format!(
                    "cloning {url} timed out after {}s and was aborted",
                    CLONE_TIMEOUT.as_secs()
                )))
            }
        }
    }

    /// Remove an installed plugin's directory.
    pub fn uninstall(&self, id: &str) -> VestaResult<()> {
        if self.record(id)?.is_none() {
            return Err(plugin_not_found(id));
        }
        std::fs::remove_dir_all(self.plugin_dir(id))?;
        Ok(())
    }
}

/// Clone one commit from `url` into `destination` and load its manifest,
/// returning the record (working tree stays at `destination` until moved).
fn clone_plugin(
    url: &str,
    destination: &Path,
    interrupt: &AtomicBool,
) -> Result<PluginRecord, String> {
    let commit = shallow_clone_commit(url, destination, interrupt)?;
    let manifest = load_manifest(destination).map_err(|error| error.to_string())?;
    let source = PluginSource {
        source_url: url.to_string(),
        commit,
        installed_at: Utc::now().to_rfc3339(),
    };
    std::fs::write(
        destination.join(SOURCE_FILE),
        source.to_toml().map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    Ok(PluginRecord { manifest, source })
}

fn shallow_clone_commit(
    url: &str,
    destination: &Path,
    interrupt: &AtomicBool,
) -> Result<String, String> {
    let parsed = gix::url::parse(url.into())
        .map_err(|error| format!("'{url}' is not a valid git URL: {error}"))?;
    ensure_supported_scheme(&parsed.scheme, url)?;

    let mut prepare = gix::clone::PrepareFetch::new(
        url,
        destination,
        gix::create::Kind::WithWorktree,
        gix::create::Options::default(),
        gix::open::Options::default(),
    )
    .map_err(|error| format!("could not start cloning {url}: {error}"))?;
    prepare = prepare.with_shallow(gix::remote::fetch::Shallow::DepthAtRemote(
        NonZeroU32::new(1).expect("shallow depth of 1 is non-zero"),
    ));
    let (mut prepare, _outcome) = prepare
        .fetch_then_checkout(gix::progress::Discard, interrupt)
        .map_err(|error| format!("could not clone {url}: {error}"))?;
    let (repo, _checkout) = prepare
        .main_worktree(gix::progress::Discard, interrupt)
        .map_err(|error| format!("could not check out {url}: {error}"))?;
    repo.head_id()
        .map(|id| id.to_string())
        .map_err(|error| format!("cloned repository has no commits: {error}"))
}

fn ensure_supported_scheme(scheme: &gix::url::Scheme, url: &str) -> Result<(), String> {
    let supported = matches!(
        scheme,
        gix::url::Scheme::Http
            | gix::url::Scheme::Https
            | gix::url::Scheme::Ssh
            | gix::url::Scheme::Git
            | gix::url::Scheme::File
    );
    if supported {
        Ok(())
    } else {
        Err(format!(
            "unsupported git URL scheme for {url}; use https://, http://, ssh:// or a local path"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn git(args: &[&str], dir: &Path) {
        let status = std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .status()
            .expect("git binary available for tests");
        assert!(status.success(), "git {args:?} failed");
    }

    /// Create a local git repository from fixture files (only tests build the
    /// fixture with the git CLI; runtime cloning is pure gix).
    fn fixture_repo(fixture: &Path, name: &str) -> PathBuf {
        let repo = std::env::temp_dir().join(format!(
            "vesta-plugin-fixture-{name}-{}",
            uuid::Uuid::new_v4().simple()
        ));
        copy_tree(fixture, &repo);
        git(&["init", "-q"], &repo);
        git(&["add", "."], &repo);
        git(
            &[
                "-c",
                "user.name=test",
                "-c",
                "user.email=test@test",
                "commit",
                "-qm",
                "fixture",
            ],
            &repo,
        );
        repo
    }

    fn copy_tree(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).unwrap();
        for entry in std::fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let target = to.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_tree(&entry.path(), &target);
            } else {
                std::fs::copy(entry.path(), target).unwrap();
            }
        }
    }

    fn manager() -> PluginManager {
        PluginManager::new(
            std::env::temp_dir().join(format!("vesta-plugins-{}", uuid::Uuid::new_v4().simple())),
        )
        .unwrap()
    }

    fn fixture_dir(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/plugins")
            .join(name)
    }

    #[tokio::test]
    async fn installs_from_local_repository_and_lists_metadata() {
        let manager = manager();
        let repo = fixture_repo(&fixture_dir("hello-world"), "hello");
        let source = repo.display().to_string();

        let record = manager.install(&source).await.unwrap();
        assert_eq!(record.manifest.id, "hello-world");
        assert_eq!(record.source.source_url, source);
        assert!(!record.source.commit.is_empty());
        assert!(!record.source.installed_at.is_empty());

        let installed = manager.root().join("hello-world");
        assert!(installed.join("index.html").is_file());
        assert!(!installed.join(".git").exists(), ".git should be stripped");
        assert!(installed.join(SOURCE_FILE).is_file());

        let listed = manager.load_all();
        assert_eq!(listed.len(), 1, "listed: {listed:?}");
        assert_eq!(listed[0].manifest.name, "Hello World");
        assert!(manager.record("hello-world").unwrap().is_some());
        assert!(manager.record("../../escape").unwrap().is_none());
        std::fs::remove_dir_all(&repo).ok();
    }

    #[tokio::test]
    async fn duplicate_install_fails_and_keeps_existing() {
        let manager = manager();
        let repo = fixture_repo(&fixture_dir("hello-world"), "dupe");
        manager.install(&repo.to_string_lossy()).await.unwrap();
        let error = manager.install(&repo.to_string_lossy()).await.unwrap_err();
        assert!(error.to_string().contains("already installed"), "{error}");
        assert_eq!(manager.load_all().len(), 1);
        std::fs::remove_dir_all(&repo).ok();
    }

    #[tokio::test]
    async fn failed_installs_leave_no_temporary_directories() {
        let manager = manager();
        let missing = std::env::temp_dir().join("vesta-plugin-missing-repo");
        let error = manager
            .install(&missing.to_string_lossy())
            .await
            .unwrap_err();
        assert!(error.to_string().contains("could not clone"), "{error}");

        // Repository with a broken manifest must also fail and clean up.
        let broken = fixture_repo(&fixture_dir("broken-manifest"), "broken");
        let error = manager
            .install(&broken.to_string_lossy())
            .await
            .unwrap_err();
        assert!(error.to_string().contains("manifest"), "{error}");
        std::fs::remove_dir_all(&broken).ok();

        let leftovers: Vec<_> = std::fs::read_dir(manager.root())
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert!(
            leftovers.is_empty(),
            "temp dirs leaked: {:?}",
            leftovers.iter().map(|e| e.file_name()).collect::<Vec<_>>()
        );
    }

    #[tokio::test]
    async fn uninstall_removes_the_plugin() {
        let manager = manager();
        let repo = fixture_repo(&fixture_dir("hello-world"), "gone");
        manager.install(&repo.to_string_lossy()).await.unwrap();
        manager.uninstall("hello-world").unwrap();
        assert!(manager.record("hello-world").unwrap().is_none());
        let missing = manager.uninstall("hello-world").unwrap_err();
        assert!(missing.to_string().contains("not found"), "{missing}");
        std::fs::remove_dir_all(&repo).ok();
    }

    #[tokio::test]
    async fn empty_url_is_rejected() {
        let manager = manager();
        let error = manager.install("   ").await.unwrap_err();
        assert!(error.to_string().contains("URL is required"), "{error}");
    }

    /// Exercises the full HTTPS clone path (rustls TLS, shallow fetch, temp
    /// cleanup) against a real remote. The remote is a plain repo without a
    /// manifest, so success here means the pipeline fails exactly at manifest
    /// validation. Skips when this environment has no outbound network;
    /// local-path installs cover the rest of the pipeline above.
    #[tokio::test]
    async fn installs_over_https_from_a_real_remote() {
        const HTTPS_URL: &str = "https://github.com/octocat/Hello-World.git";
        let manager = manager();
        match tokio::time::timeout(
            std::time::Duration::from_secs(90),
            manager.install(HTTPS_URL),
        )
        .await
        {
            Err(_) => {
                // A 90s timeout is far beyond any healthy clone of this repo;
                // treat it like an offline environment.
                eprintln!("skipping: network unreachable");
            }
            Ok(Err(error)) => {
                let message = error.to_string();
                if message.contains("could not clone")
                    && (message.contains("connect")
                        || message.contains("resolve")
                        || message.contains("execution")
                        || message.to_lowercase().contains("network")
                        || message.contains("timeout"))
                {
                    // Offline environment: skip.
                    eprintln!("skipping: network unreachable ({message})");
                } else if message.contains("missing or unreadable")
                    && message.contains(crate::plugins::manifest::MANIFEST_FILE)
                {
                    // Hello-World is a normal repo, not a Vesta plugin. Reaching
                    // the manifest check proves the HTTPS clone, checkout, and
                    // temp-directory handling all worked over the network.
                } else {
                    panic!("unexpected install failure: {error}");
                }
            }
            Ok(Ok(record)) => {
                assert_eq!(record.source.source_url, HTTPS_URL);
                assert!(!record.source.commit.is_empty());
                let installed = manager.root().join("Hello-World");
                assert!(installed.is_dir());
                manager.uninstall("Hello-World").unwrap();
            }
        }
    }
}
