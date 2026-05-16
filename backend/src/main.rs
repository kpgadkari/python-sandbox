use std::{env, fs, net::SocketAddr, path::PathBuf};

use anyhow::Context;
use sqlx::mysql::MySqlPoolOptions;

mod auth;
mod db;
mod error;
mod lessons;
mod models;
mod project_files;
mod projects;
mod routes;
mod state;
#[cfg(test)]
mod test_db;

use crate::{
    db::{seed_lessons, seed_users},
    routes::build_router,
    state::AppState,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let data_dir = PathBuf::from(env::var("SANDBOX_DATA_DIR").unwrap_or_else(|_| "./data".into()));
    fs::create_dir_all(data_dir.join("projects")).context("create data directories")?;

    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "mysql://sandbox:sandbox@127.0.0.1:3306/python_sandbox".into());
    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .context("connect to mysql database")?;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("run database migrations")?;
    seed_users(&pool).await?;
    seed_lessons(&pool).await?;

    let state = AppState { db: pool, data_dir };

    let bind = env::var("SANDBOX_BIND").unwrap_or_else(|_| "127.0.0.1:8080".into());
    let addr: SocketAddr = bind.parse().context("parse SANDBOX_BIND")?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("listening on {addr}");
    axum::serve(listener, build_router(state))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;

    Ok(())
}
