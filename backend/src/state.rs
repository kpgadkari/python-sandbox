use std::path::PathBuf;

use sqlx::MySqlPool;

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) db: MySqlPool,
    pub(crate) data_dir: PathBuf,
}
