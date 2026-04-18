use anyhow::Result;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    ConnectOptions, SqlitePool,
};
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

#[derive(Clone)]
pub struct Db {
    pool: SqlitePool,
}

impl Db {
    pub async fn new(path: Option<PathBuf>) -> Result<Self> {
        let db_path = match path {
            Some(p) => p,
            None => {
                let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("no home dir"))?;
                home.join(".local/state/hangar/hangar.db")
            }
        };

        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Per-connection options. A busy_timeout is essential: without it
        // any concurrent writer (e.g. the event persister task) racing with
        // a multi-statement write transaction (e.g. DELETE /sessions/:id)
        // returns SQLITE_BUSY immediately.
        let url = format!("sqlite:{}?mode=rwc", db_path.display());
        let connect_opts = SqliteConnectOptions::from_str(&url)?
            .busy_timeout(Duration::from_secs(5))
            .disable_statement_logging();

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(connect_opts)
            .await?;

        sqlx::query("PRAGMA journal_mode=WAL")
            .execute(&pool)
            .await?;

        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(Db { pool })
    }

    pub async fn new_in_memory() -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;

        sqlx::query("PRAGMA journal_mode=WAL")
            .execute(&pool)
            .await?;

        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(Db { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}
