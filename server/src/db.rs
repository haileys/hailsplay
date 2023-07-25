pub mod types;

use std::{path::Path, collections::HashSet};
use anyhow::{Result, Context};
use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};

pub type Pool = SqlitePool;

static MIGRATIONS: &[(i64, &str)] = &[
    (001, include_str!("../migrations/001_create_db.sql")),
];

pub async fn open(path: &Path) -> Result<Pool> {
    log::info!("Opening database at {}", path.display());

    let options = SqliteConnectOptions::new()
        .filename(path)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .create_if_missing(true);

    let pool = SqlitePool::connect_with(options).await?;

    let mut txn = pool.begin().await?;

    sqlx::query("CREATE TABLE IF NOT EXISTS _migrations (rowid INTEGER PRIMARY KEY);")
        .execute(&mut *txn)
        .await
        .context("create migrations table")?;

    let db_versions: HashSet<i64> = sqlx::query_as("SELECT rowid FROM _migrations")
        .fetch_all(&mut *txn)
        .await
        .context("query migration versions")?
        .into_iter()
        .map(|(row,)| row)
        .collect();

    for (ver, sql) in MIGRATIONS {
        if db_versions.contains(ver) {
            continue;
        }

        // let sql_stmts = sql_stmts.split(";");

        // for stmt in sql_stmts {
            // let sql = stmt.trim();

            sqlx::query(sql)
                .execute(&mut *txn)
                .await
                .with_context(|| format!("Running migration {ver}: {sql}"))?;
        // }

        sqlx::query("INSERT INTO _migrations (rowid) VALUES ($1)")
            .bind(ver)
            .execute(&mut *txn)
            .await?;
    }

    txn.commit().await?;

    Ok(pool)
}
