pub mod archive;
pub mod asset;
pub mod radio;
pub mod schema;
pub mod types;

mod migrate;

use std::path::Path;
use std::sync::Arc;

use diesel::OptionalExtension;
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

#[derive(Error, Debug)]
pub enum Error {
    #[error("opening connection: {0}")]
    Pool(#[from] r2d2::PoolError),
    #[error("query error: {0}")]
    Query(#[from] diesel::result::Error),
}

pub trait OptionalExt<T> {
    fn optional(self) -> Result<Option<T>, Error>;
}

impl<T> OptionalExt<T> for Result<T, Error> {
    fn optional(self) -> Result<Option<T>, Error> {
        match self {
            Ok(val) => Ok(Some(val)),
            Err(Error::Pool(e)) => Err(Error::Pool(e)),
            Err(Error::Query(e)) => Err(e).optional().map_err(Error::Query),
        }
    }
}

impl Pool {
    pub async fn get(&self) -> MutexGuard<'_, rusqlite::Connection> {
        self.conn.lock().await
    }

    pub async fn with<R>(&self, f: impl FnOnce(&mut rusqlite::Connection) -> R) -> R {
        let mut conn = self.get().await;
        tokio::task::block_in_place(|| f(&mut conn))
    }

    pub async fn diesel<T, E>(
        &self,
        func: impl FnOnce(&mut Connection) -> Result<T, E>,
    ) -> Result<T, E>
        where E: From<Error>
    {
        tokio::task::block_in_place(|| {
            let mut conn = self.pool.get().map_err(Error::Pool)?;
            let ret = func(&mut conn)?;
            Ok(ret)
        })
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
