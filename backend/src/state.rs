use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use rusqlite::Connection;

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) db: Arc<Mutex<Connection>>,
    pub(crate) data_dir: PathBuf,
}
