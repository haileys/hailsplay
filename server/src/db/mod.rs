pub mod archive;
pub mod asset;
pub mod radio;

mod migrate;

use std::path::Path;
use std::sync::Arc;

use rusqlite::Connection;
use thiserror::Error;
use tokio::sync::{Mutex, MutexGuard};

#[derive(Clone)]
pub struct Pool {
    conn: Arc<Mutex<Connection>>,
}

impl Pool {
    pub async fn get(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().await
    }

    pub async fn with<R>(&self, f: impl FnOnce(&mut rusqlite::Connection) -> R) -> R {
        let mut conn = self.get().await;
        tokio::task::block_in_place(|| f(&mut conn))
    }
}

#[derive(Debug, Error)]
pub enum OpenError {
    #[error("database error: {0}")]
    Open(#[from] rusqlite::Error),

    #[error("running migrations: {0}")]
    Migration(#[from] migrate::MigrationError),
}

pub async fn open(path: &Path) -> Result<Pool, OpenError> {
    tokio::task::block_in_place(|| {
        let mut conn = Connection::open(path)?;

        // migrate database on every open
        migrate::run(&mut conn)?;

        Ok(Pool { conn: Arc::new(Mutex::new(conn)) })
    })
}
