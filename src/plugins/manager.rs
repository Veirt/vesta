use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use chrono::Utc;
use tokio::sync::Mutex;
use tokio::task::JoinSet;

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
    /// Serialize installs, updates, and uninstalls so simultaneous mutations
    /// of the plugins directory cannot race.
    gate: Mutex<()>,
    /// Owned cleanup tasks for timed-out clone workers so none outlive `self`.
    cleanup_tasks: std::sync::Mutex<JoinSet<()>>,
}

impl Drop for PluginManager {
    fn drop(&mut self) {
        if let Ok(mut tasks) = self.cleanup_tasks.lock() {
            tasks.abort_all();
        }
    }
}

fn plugin_error(message: impl Into<String>) -> VestaError {
    VestaError::Plugin(message.into())
}

fn plugin_not_found(id: &str) -> VestaError {
    VestaError::PluginNotFound { id: id.to_string() }
}

fn plugin_dir_in(root: &Path, id: &str) -> VestaResult<PathBuf> {
    if !is_valid_slug(id) {
        return Err(plugin_not_found(id));
    }
    Ok(root.join(id))
}

fn load_record_in(root: &Path, id: &str) -> VestaResult<Option<PluginRecord>> {
    if !is_valid_slug(id) {
        return Ok(None);
    }
    let dir = plugin_dir_in(root, id)?;
    if !dir.is_dir() {
        return Ok(None);
    }
    let manifest = load_manifest(&dir)?;
    let source = PluginSource::from_toml(&std::fs::read_to_string(dir.join(SOURCE_FILE))?)?;
    Ok(Some(PluginRecord { manifest, source }))
}

impl PluginManager {
    pub fn new(root: impl Into<PathBuf>) -> VestaResult<Self> {
        let root = root.into();
        std::fs::create_dir_all(&root)?;
        Ok(Self {
            root,
            gate: Mutex::new(()),
            cleanup_tasks: std::sync::Mutex::new(JoinSet::new()),
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
        load_record_in(&self.root, id)
    }

    /// Directory for one installed plugin. This is the single choke point
    /// where a plugin id becomes a filesystem path, so non-slug ids — which
    /// could otherwise escape the plugins root via `..` — are rejected here
    /// for every caller.
    pub fn plugin_dir(&self, id: &str) -> VestaResult<PathBuf> {
        plugin_dir_in(&self.root, id)
    }

    /// Shallow-clone `url`, validate its manifest, strip `.git`, then move the
    /// tree into place under the manifest's plugin id.
    pub async fn install(&self, url: &str) -> VestaResult<PluginRecord> {
        if url.trim().is_empty() {
            return Err(plugin_error("a git repository URL is required"));
        }
        let _gate = self.gate.lock().await;
        let (temp_dir, record) = self.fetch_plugin(url).await?;
        let root = self.root.clone();
        tokio::task::spawn_blocking(move || {
            let destination = plugin_dir_in(&root, &record.manifest.id)?;
            if destination.exists() {
                std::fs::remove_dir_all(&temp_dir).ok();
                return Err(plugin_error(format!(
                    "plugin '{}' is already installed: remove it before installing again",
                    record.manifest.id
                )));
            }
            if let Err(error) = std::fs::rename(&temp_dir, &destination) {
                std::fs::remove_dir_all(&temp_dir).ok();
                return Err(error.into());
            }
            Ok(record)
        })
        .await
        .map_err(|error| plugin_error(format!("plugin install task failed: {error}")))?
    }

    /// Re-fetch an installed plugin from its recorded source URL and swap the
    /// tree in place. The fetched manifest must declare the same plugin id:
    /// an id change means a different plugin, which must be installed
    /// separately. Dashboard tiles keep working — the registry rebuild after
    /// the swap picks up the new widget specs, and old tiles fail validation
    /// only if their fields no longer exist.
    pub async fn update(&self, id: &str) -> VestaResult<PluginRecord> {
        let _gate = self.gate.lock().await;
        let root = self.root.clone();
        let id_owned = id.to_string();
        let current = tokio::task::spawn_blocking(move || load_record_in(&root, &id_owned))
            .await
            .map_err(|error| plugin_error(format!("plugin read task failed: {error}")))??
            .ok_or_else(|| plugin_not_found(id))?;
        let (temp_dir, fresh) = self.fetch_plugin(&current.source.source_url).await?;
        let root = self.root.clone();
        let id_owned = id.to_string();
        tokio::task::spawn_blocking(move || {
            if fresh.manifest.id != current.manifest.id {
                std::fs::remove_dir_all(&temp_dir).ok();
                return Err(plugin_error(format!(
                    "updated manifest declares plugin id '{}' but '{}' is installed; install it under the new id instead",
                    fresh.manifest.id, current.manifest.id
                )));
            }
            let destination = plugin_dir_in(&root, &id_owned)?;
            let backup = root.join(format!(".old-{}", uuid::Uuid::new_v4().simple()));
            // Swap with rollback: the installed tree returns if the new one
            // cannot move into place.
            if let Err(error) = std::fs::rename(&destination, &backup)
                .and_then(|()| std::fs::rename(&temp_dir, &destination))
            {
                std::fs::rename(&backup, &destination).ok();
                std::fs::remove_dir_all(&temp_dir).ok();
                return Err(error.into());
            }
            std::fs::remove_dir_all(&backup).ok();
            Ok(fresh)
        })
        .await
        .map_err(|error| plugin_error(format!("plugin update task failed: {error}")))?
    }

    /// Remove an installed plugin's directory under the mutation gate.
    /// Checks only directory existence after slug validation so corrupt plugins
    /// with broken manifests can still be uninstalled.
    pub async fn uninstall(&self, id: &str) -> VestaResult<()> {
        let _gate = self.gate.lock().await;
        let root = self.root.clone();
        let id_owned = id.to_string();
        tokio::task::spawn_blocking(move || {
            let dir = plugin_dir_in(&root, &id_owned)?;
            if !dir.is_dir() {
                return Err(plugin_not_found(&id_owned));
            }
            std::fs::remove_dir_all(&dir)?;
            Ok(())
        })
        .await
        .map_err(|error| plugin_error(format!("plugin uninstall task failed: {error}")))?
    }

    /// Clone `url` into a hidden temporary directory under the plugins root
    /// and validate its manifest. Returns the record and the temporary path;
    /// the caller moves the tree and owns its cleanup. The clone runs on a
    /// blocking worker with a shared interrupt flag; on timeout the worker is
    /// told to stop and its temporary directory cleaned up once it exits.
    async fn fetch_plugin(&self, url: &str) -> VestaResult<(PathBuf, PluginRecord)> {
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
                let git_dir = temp_dir.join(".git");
                tokio::task::spawn_blocking(move || std::fs::remove_dir_all(git_dir))
                    .await
                    .ok();
                Ok((temp_dir, record))
            }
            Ok(Ok(Err(message))) => {
                let dir = temp_dir.clone();
                tokio::task::spawn_blocking(move || std::fs::remove_dir_all(dir))
                    .await
                    .ok();
                Err(plugin_error(message))
            }
            Ok(Err(join)) => {
                let dir = temp_dir.clone();
                tokio::task::spawn_blocking(move || std::fs::remove_dir_all(dir))
                    .await
                    .ok();
                Err(plugin_error(format!("clone worker failed: {join}")))
            }
            Err(_elapsed) => {
                interrupt.store(true, Ordering::Relaxed);
                let cleanup_dir = temp_dir;
                if let Ok(mut tasks) = self.cleanup_tasks.lock() {
                    while tasks.try_join_next().is_some() {}
                    tasks.spawn(async move {
                        let _ = pinned.await;
                        let _ = tokio::task::spawn_blocking(move || {
                            std::fs::remove_dir_all(&cleanup_dir)
                        })
                        .await;
                    });
                }
                Err(plugin_error(format!(
                    "cloning {url} timed out after {}s and was aborted",
                    CLONE_TIMEOUT.as_secs()
                )))
            }
        }
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
    use crate::plugins::test_fixtures::{TempDir, fixture_dir, git, git_fixture_repo, temp_dir};

    fn test_manager() -> (TempDir, PluginManager) {
        let dir = temp_dir("vesta-plugins");
        let manager = PluginManager::new(dir.path()).unwrap();
        (dir, manager)
    }

    #[tokio::test]
    async fn installs_from_local_repository_and_lists_metadata() {
        let (_dir, manager) = test_manager();
        let repo = git_fixture_repo(&fixture_dir("hello-world"), "hello");
        let source = repo.path().display().to_string();

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
    }

    #[tokio::test]
    async fn duplicate_install_fails_and_keeps_existing() {
        let (_dir, manager) = test_manager();
        let repo = git_fixture_repo(&fixture_dir("hello-world"), "dupe");
        manager
            .install(&repo.path().to_string_lossy())
            .await
            .unwrap();
        let error = manager
            .install(&repo.path().to_string_lossy())
            .await
            .unwrap_err();
        assert!(error.to_string().contains("already installed"), "{error}");
        assert_eq!(manager.load_all().len(), 1);
    }

    #[tokio::test]
    async fn failed_installs_leave_no_temporary_directories() {
        let (_dir, manager) = test_manager();
        let missing = std::env::temp_dir().join("vesta-plugin-missing-repo");
        let error = manager
            .install(&missing.to_string_lossy())
            .await
            .unwrap_err();
        assert!(error.to_string().contains("could not clone"), "{error}");

        // Repository with a broken manifest must also fail and clean up.
        let broken = git_fixture_repo(&fixture_dir("broken-manifest"), "broken");
        let error = manager
            .install(&broken.path().to_string_lossy())
            .await
            .unwrap_err();
        assert!(error.to_string().contains("manifest"), "{error}");

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
        let (_dir, manager) = test_manager();
        let repo = git_fixture_repo(&fixture_dir("hello-world"), "gone");
        manager
            .install(&repo.path().to_string_lossy())
            .await
            .unwrap();
        manager.uninstall("hello-world").await.unwrap();
        assert!(manager.record("hello-world").unwrap().is_none());
        let missing = manager.uninstall("hello-world").await.unwrap_err();
        assert!(missing.to_string().contains("not found"), "{missing}");
    }

    #[tokio::test]
    async fn uninstall_removes_corrupt_plugin_directory() {
        let (_dir, manager) = test_manager();
        let broken_dir = manager.root().join("broken-plugin");
        std::fs::create_dir_all(&broken_dir).unwrap();
        std::fs::write(broken_dir.join("vesta-plugin.toml"), "not = [valid toml").unwrap();
        manager.uninstall("broken-plugin").await.unwrap();
        assert!(!broken_dir.exists());
    }

    #[tokio::test]
    async fn empty_url_is_rejected() {
        let (_dir, manager) = test_manager();
        let error = manager.install("   ").await.unwrap_err();
        assert!(error.to_string().contains("URL is required"), "{error}");
    }

    /// `plugin_dir` is the only place a plugin id becomes a path, so it must
    /// refuse anything that could escape the plugins root.
    #[test]
    fn plugin_dir_rejects_non_slug_ids() {
        let (_dir, manager) = test_manager();
        for bad in ["../escape", "C:\\temp", "Bad-Id", "a/b", ""] {
            assert!(
                manager.plugin_dir(bad).is_err(),
                "expected plugin_dir('{bad}') rejection"
            );
        }
        assert!(manager.plugin_dir("hello-world").is_ok());
    }

    #[tokio::test]
    async fn update_refetches_and_swaps_the_plugin_tree() {
        let (_dir, manager) = test_manager();
        let repo = git_fixture_repo(&fixture_dir("hello-world"), "update");
        manager
            .install(&repo.path().to_string_lossy())
            .await
            .unwrap();
        let old_commit = manager
            .record("hello-world")
            .unwrap()
            .unwrap()
            .source
            .commit;

        // Bump the version in the remote repository, then update in place.
        let manifest_path = repo.path().join(crate::plugins::manifest::MANIFEST_FILE);
        let manifest = std::fs::read_to_string(&manifest_path).unwrap();
        std::fs::write(
            &manifest_path,
            manifest.replace("version = \"1.0.0\"", "version = \"1.0.1\""),
        )
        .unwrap();
        git(&["add", "."], repo.path());
        git(
            &[
                "-c",
                "user.name=test",
                "-c",
                "user.email=test@test",
                "commit",
                "-qm",
                "bump",
            ],
            repo.path(),
        );

        let updated = manager.update("hello-world").await.unwrap();
        assert_eq!(updated.manifest.version.as_deref(), Some("1.0.1"));
        assert_ne!(updated.source.commit, old_commit);
        assert!(
            manager
                .plugin_dir("hello-world")
                .unwrap()
                .join("index.html")
                .is_file()
        );
        assert!(manager.record("hello-world").unwrap().is_some());

        // The swap left no backup or temporary trees behind.
        let leftovers: Vec<_> = std::fs::read_dir(manager.root())
            .unwrap()
            .map(Result::unwrap)
            .filter(|entry| entry.file_name().to_string_lossy().starts_with('.'))
            .collect();
        assert!(leftovers.is_empty(), "swap leftovers: {leftovers:?}");
    }

    /// A new manifest id is a different plugin, not an update; the installed
    /// tree must stay untouched.
    #[tokio::test]
    async fn update_refuses_a_changed_plugin_id_and_keeps_the_installed_tree() {
        let (_dir, manager) = test_manager();
        let repo = git_fixture_repo(&fixture_dir("hello-world"), "id-change");
        manager
            .install(&repo.path().to_string_lossy())
            .await
            .unwrap();

        let manifest_path = repo.path().join(crate::plugins::manifest::MANIFEST_FILE);
        let manifest = std::fs::read_to_string(&manifest_path).unwrap();
        std::fs::write(
            &manifest_path,
            manifest.replace("id = \"hello-world\"", "id = \"renamed-plugin\""),
        )
        .unwrap();
        git(&["add", "."], repo.path());
        git(
            &[
                "-c",
                "user.name=test",
                "-c",
                "user.email=test@test",
                "commit",
                "-qm",
                "rename",
            ],
            repo.path(),
        );

        let error = manager.update("hello-world").await.unwrap_err();
        assert!(
            error.to_string().contains("renamed-plugin"),
            "expected id mismatch, got: {error}"
        );
        assert!(manager.record("hello-world").unwrap().is_some());
        assert!(manager.record("renamed-plugin").unwrap().is_none());
    }

    #[tokio::test]
    async fn update_unknown_id_is_not_found() {
        let (_dir, manager) = test_manager();
        let error = manager.update("ghost").await.unwrap_err();
        assert!(error.to_string().contains("not found"), "{error}");
    }
}
