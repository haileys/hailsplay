pub mod asset;
pub mod radio;

mod migrate;

use std::path::Path;

use rusqlite::Connection;
use thiserror::Error;
use tokio::sync::{Mutex, MutexGuard};

pub struct Pool {
    conn: Mutex<Connection>,
}

impl Pool {
    pub async fn get(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().await
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

        Ok(Pool { conn: Mutex::new(conn) })
    })
}
