use std::path::Path;
use std::time::Duration;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Row, SqlitePool};

use crate::error::VestaError;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!();

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

#[derive(Clone, Debug)]
pub struct StoredIcon {
    pub content_type: String,
    pub data: Vec<u8>,
}

impl Database {
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, VestaError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .busy_timeout(Duration::from_secs(5));
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;
        MIGRATOR.run(&pool).await?;

        Ok(Self { pool })
    }

    pub async fn store_icon(
        &self,
        name: &str,
        content_type: &str,
        data: &[u8],
    ) -> Result<i64, VestaError> {
        let result = sqlx::query("INSERT INTO icons (name, content_type, data) VALUES (?, ?, ?)")
            .bind(name)
            .bind(content_type)
            .bind(data)
            .execute(&self.pool)
            .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn load_icon(&self, id: i64) -> Result<Option<StoredIcon>, VestaError> {
        let row = sqlx::query("SELECT content_type, data FROM icons WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|row| StoredIcon {
            content_type: row.get("content_type"),
            data: row.get("data"),
        }))
    }

    pub async fn admin_password_hash(&self) -> Result<Option<String>, VestaError> {
        Ok(
            sqlx::query_scalar("SELECT password_hash FROM admin_account WHERE id = 1")
                .fetch_optional(&self.pool)
                .await?,
        )
    }

    pub async fn create_admin(&self, password_hash: &str) -> Result<bool, VestaError> {
        let result = sqlx::query(
            "INSERT INTO admin_account (id, password_hash) VALUES (1, ?) \
             ON CONFLICT(id) DO NOTHING",
        )
        .bind(password_hash)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn store_session(&self, token_hash: &str, expires_at: i64) -> Result<(), VestaError> {
        sqlx::query("DELETE FROM auth_sessions WHERE expires_at <= unixepoch()")
            .execute(&self.pool)
            .await?;
        sqlx::query("INSERT INTO auth_sessions (token_hash, expires_at) VALUES (?, ?)")
            .bind(token_hash)
            .bind(expires_at)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn session_is_valid(&self, token_hash: &str) -> Result<bool, VestaError> {
        let valid: i64 = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM auth_sessions WHERE token_hash = ? AND expires_at > unixepoch())",
        )
        .bind(token_hash)
        .fetch_one(&self.pool)
        .await?;
        Ok(valid == 1)
    }

    pub async fn delete_session(&self, token_hash: &str) -> Result<(), VestaError> {
        sqlx::query("DELETE FROM auth_sessions WHERE token_hash = ?")
            .bind(token_hash)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn uploaded_icon_blobs_round_trip_through_database() {
        let path = database_path("icon-round-trip");
        let database = Database::open(&path).await.unwrap();
        let icon_id = database
            .store_icon("anime.webp", "image/webp", &[82, 73, 70, 70])
            .await
            .unwrap();
        let icon = database.load_icon(icon_id).await.unwrap().unwrap();
        assert_eq!(icon.data, [82, 73, 70, 70]);

        drop(database);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("sqlite-wal"));
        let _ = std::fs::remove_file(path.with_extension("sqlite-shm"));
    }

    #[tokio::test]
    async fn dashboard_tables_are_removed_and_migrations_reopen_cleanly() {
        let path = database_path("settings-cleanup");
        let initial_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(&path)
                    .create_if_missing(true)
                    .foreign_keys(true),
            )
            .await
            .unwrap();
        MIGRATOR.run_to(1, &initial_pool).await.unwrap();
        drop(initial_pool);

        // This is the upgrade path for databases that applied 0001 before 0002
        // was introduced. Opening verifies the original checksum, then cleans
        // up the obsolete tables with the new migration.
        let database = Database::open(&path).await.unwrap();
        let remaining: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' \
             AND name IN ('app_settings', 'nodes', 'service_groups', 'services')",
        )
        .fetch_one(&database.pool)
        .await
        .unwrap();
        assert_eq!(remaining, 0);
        drop(database);

        let reopened = Database::open(&path).await.unwrap();
        drop(reopened);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("sqlite-wal"));
        let _ = std::fs::remove_file(path.with_extension("sqlite-shm"));
    }

    fn database_path(name: &str) -> std::path::PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "vesta-{name}-{}-{unique}.sqlite",
            std::process::id()
        ))
    }
}
