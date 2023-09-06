use std::collections::BTreeSet;
use std::path::Path;

use thiserror::Error;
use rusqlite::Connection;
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
    Migration(#[from] MigrationError),
}

pub async fn open(path: &Path) -> Result<Pool, OpenError> {
    tokio::task::block_in_place(|| {
        let mut conn = Connection::open(path)?;

        // migrate database on every open
        migrate(&mut conn)?;

        Ok(Pool { conn: Mutex::new(conn) })
    })
}

#[derive(Debug, Error)]
pub enum MigrationError {
    #[error("unknown versions in database: {0}")]
    UnknownMigrationsInDatabase(String),

    #[error("failed to apply {0}: {1}")]
    FailedToApplyMigration(String, rusqlite::Error),

    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
}

fn migrate(conn: &mut Connection) -> Result<(), MigrationError> {
    let txn = conn.transaction()?;

    txn.execute_batch("CREATE TABLE IF NOT EXISTS schema_migrations (version TEXT PRIMARY KEY);")?;

    let mut database_versions = txn
        .prepare("SELECT version FROM schema_migrations ORDER BY version ASC")?
        .query_map([], |row| row.get(0))?
        .collect::<Result<BTreeSet<String>, _>>()?;

    // run all migrations in transaction
    for (name, sql) in MIGRATIONS {
        let (version, _description) = name.split_once("_")
            .expect("migration name contains no _");

        if database_versions.remove(version) {
            // migration already in database
            continue;
        }

        txn.execute_batch(sql).map_err(|err|
            MigrationError::FailedToApplyMigration(name.to_string(), err))?;

        txn.execute("INSERT INTO schema_migrations (version) VALUES (?1)", [version])?;
    }

    // if database_versions is not empty by the end, there are unknown
    // migrations in the database, fail and rollback
    if !database_versions.is_empty() {
        let versions = database_versions.into_iter().collect::<Vec<_>>();
        return Err(MigrationError::UnknownMigrationsInDatabase(versions.join(", ")))
    }

    // commit transaction, we are done!
    txn.commit()?;
    Ok(())
}

macro_rules! migration {
    ($name:literal) => { ($name, include_str!(concat!("../migrations/", $name, ".sql"))) }
}

static MIGRATIONS: &[(&str, &str)] = &[
    migration!("000_create_schema"),
];
