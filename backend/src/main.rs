use std::{
    env, fs,
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use anyhow::Context;
use rusqlite::Connection;

mod auth;
mod db;
mod error;
mod lessons;
mod models;
mod project_files;
mod projects;
mod routes;
mod state;

use crate::{
    db::{init_db, seed_lessons, seed_user},
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

    let db_path = PathBuf::from(
        env::var("SANDBOX_DB_PATH")
            .unwrap_or_else(|_| data_dir.join("sandbox.db").display().to_string()),
    );
    let connection = Connection::open(db_path).context("open sqlite database")?;
    init_db(&connection)?;
    seed_user(&connection)?;
    seed_lessons(&connection)?;

    let state = AppState {
        db: Arc::new(Mutex::new(connection)),
        data_dir,
    };

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
