use std::collections::BTreeSet;

use diesel::{Connection, connection::SimpleConnection};
use diesel::prelude::*;
use thiserror::Error;

use crate::db;
use crate::db::schema::schema_migrations;

#[derive(Debug, Error)]
pub enum MigrationError {
    #[error("unknown versions in database: {0}")]
    UnknownMigrationsInDatabase(String),

    #[error("failed to apply {0}: {1}")]
    FailedToApplyMigration(String, diesel::result::Error),

    #[error("database error: {0}")]
    Database(#[from] diesel::result::Error),
}

pub fn run(conn: &mut db::Connection) -> Result<(), MigrationError> {
    conn.transaction(|conn| {
        conn.batch_execute(r"CREATE TABLE IF NOT EXISTS schema_migrations (version TEXT PRIMARY KEY);")?;

        let mut database_versions = schema_migrations::table
            .select(schema_migrations::version)
            .load::<Option<String>>(conn)?
            .into_iter()
            .filter_map(|ver| ver)
            .collect::<BTreeSet<_>>();

        // run all migrations in transaction
        for (name, sql) in MIGRATIONS {
            let (version, _description) = name.split_once("_")
                .expect("migration name contains no _");

            if database_versions.remove(version) {
                // migration already in database
                continue;
            }

            conn.batch_execute(sql).map_err(|err|
                MigrationError::FailedToApplyMigration(name.to_string(), err))?;

            diesel::insert_into(schema_migrations::table)
                .values(&Version { version })
                .execute(conn)?;
        }

        // if database_versions is not empty by the end, there are unknown
        // migrations in the database, fail and rollback
        if !database_versions.is_empty() {
            let versions = database_versions.into_iter().collect::<Vec<_>>();
            return Err(MigrationError::UnknownMigrationsInDatabase(versions.join(", ")))
        }

        Ok(())
    })
}

#[derive(Insertable)]
#[diesel(table_name = schema_migrations)]
struct Version<'a> {
    version: &'a str,
}

macro_rules! migration {
    ($name:literal) => { ($name, include_str!(concat!("../../migrations/", $name, ".sql"))) }
}

static MIGRATIONS: &[(&str, &str)] = &[
    migration!("000_create_schema"),
    migration!("001_create_archived_media"),
];
