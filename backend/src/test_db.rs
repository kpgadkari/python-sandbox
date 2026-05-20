use std::{env, sync::Arc};

use anyhow::Context;
use sqlx::MySqlPool;
use tokio::sync::{Mutex, OwnedMutexGuard};

static TEST_DB_LOCK: std::sync::OnceLock<Arc<Mutex<()>>> = std::sync::OnceLock::new();

pub(crate) struct TestDb {
    pub(crate) pool: MySqlPool,
    _guard: OwnedMutexGuard<()>,
}

impl TestDb {
    pub(crate) async fn connect() -> anyhow::Result<Option<Self>> {
        let Some(database_url) = test_database_url()? else {
            return Ok(None);
        };
        let guard = TEST_DB_LOCK
            .get_or_init(|| Arc::new(Mutex::new(())))
            .clone()
            .lock_owned()
            .await;
        let pool = MySqlPool::connect(&database_url)
            .await
            .context("connect to test mariadb database")?;
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .context("run test database migrations")?;
        reset_database(&pool).await?;
        Ok(Some(Self {
            pool,
            _guard: guard,
        }))
    }
}

fn test_database_url() -> anyhow::Result<Option<String>> {
    match env::var("SANDBOX_TEST_DATABASE_URL") {
        Ok(value) => Ok(Some(value)),
        Err(_) if env::var_os("CI").is_some() => {
            anyhow::bail!("SANDBOX_TEST_DATABASE_URL must be set in CI")
        }
        Err(_) => {
            eprintln!("skipping MariaDB integration test; set SANDBOX_TEST_DATABASE_URL to run it");
            Ok(None)
        }
    }
}

async fn reset_database(pool: &MySqlPool) -> anyhow::Result<()> {
    sqlx::query("SET FOREIGN_KEY_CHECKS = 0").execute(pool).await?;
    sqlx::query("DELETE FROM attempts").execute(pool).await?;
    sqlx::query("DELETE FROM sessions").execute(pool).await?;
    sqlx::query("DELETE FROM projects").execute(pool).await?;
    sqlx::query("DELETE FROM lessons").execute(pool).await?;
    sqlx::query("DELETE FROM users").execute(pool).await?;
    sqlx::query("SET FOREIGN_KEY_CHECKS = 1").execute(pool).await?;
    Ok(())
}
