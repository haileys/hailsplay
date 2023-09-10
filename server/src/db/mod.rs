pub mod archive;
pub mod asset;
pub mod radio;
pub mod schema;
pub mod types;

mod migrate;

use std::path::Path;
use std::sync::Arc;

use thiserror::Error;
use tokio::sync::{Mutex, MutexGuard};

use diesel::sqlite::SqliteConnection;
use diesel::r2d2::{self, ConnectionManager};

pub type R2D2Pool = r2d2::Pool<ConnectionManager<SqliteConnection>>;

pub type Connection = r2d2::PooledConnection<ConnectionManager<SqliteConnection>>;

#[derive(Clone)]
pub struct Pool {
    conn: Arc<Mutex<rusqlite::Connection>>,
    #[allow(unused)]
    pool: R2D2Pool,
}

impl Pool {
    pub async fn get(&self) -> MutexGuard<'_, rusqlite::Connection> {
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

    #[error("opening database: {0}")]
    Open2(#[from] r2d2::PoolError),

    #[error("database error: {0}")]
    Pool(#[from] diesel::r2d2::Error),

    #[error("database path not utf-8")]
    PathNotUtf8,

    #[error("running migrations: {0}")]
    Migrate(#[from] migrate::MigrationError),
}

pub async fn open(path: &Path) -> Result<Pool, OpenError> {
    let path = path.to_str()
        .ok_or(OpenError::PathNotUtf8)?;

    tokio::task::block_in_place(|| {
        let manager = ConnectionManager::<SqliteConnection>::new(path);

        let pool = r2d2::Pool::builder()
            .build(manager)?;

        let mut conn = pool.get()?;

        migrate::run(&mut conn)?;

        let conn = rusqlite::Connection::open(path)?;

        Ok(Pool {
            conn: Arc::new(Mutex::new(conn)),
            pool,
        })
    })
}
