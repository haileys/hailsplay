use std::marker::PhantomData;

use chrono::{DateTime, Utc, FixedOffset};
use derive_more::{From, Into};
use diesel::backend::Backend;
use diesel::deserialize::{FromSqlRow, FromSql, self};
use diesel::sql_types::Text;
use diesel::{prelude::*, serialize};
use diesel::serialize::{ToSql, IsNull};
use diesel::sqlite::Sqlite;
use diesel::{AsExpression, sql_types};
use serde::{Serialize};
use serde::de::DeserializeOwned;

#[derive(Debug, Clone, FromSqlRow, AsExpression, From, Into)]
#[diesel(sql_type = sql_types::Text)]
pub struct Url(pub url::Url);

#[derive(Debug, Clone, From, Into)]
pub struct TimestampUtc(pub DateTime<Utc>);

impl FromSql<Text, Sqlite> for TimestampUtc {
    fn from_sql(value: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let text = <String as FromSql<Text, Sqlite>>::from_sql(value)?;
        let time = DateTime::<FixedOffset>::parse_from_rfc3339(&text)?;
        let utc = time.with_timezone(&Utc);
        Ok(TimestampUtc(utc))
    }
}

impl ToSql<Text, Sqlite> for TimestampUtc {
    fn to_sql<'b>(&'b self, out: &mut serialize::Output<'b, '_, Sqlite>) -> serialize::Result {
        let text: String = self.0.to_rfc3339();
        out.set_value(text);
        Ok(IsNull::No)
    }
}

impl Expression for TimestampUtc {
    type SqlType = Text;
}

#[derive(Debug, Clone, FromSqlRow, AsExpression)]
#[diesel(sql_type = sql_types::Text)]
pub struct Json<T: DeserializeOwned + Serialize> {
    json: String,
    _phantom: PhantomData<T>,
}

impl<T: DeserializeOwned + Serialize> Json<T> {
    pub fn new(val: &T) -> Self {
        let json = serde_json::to_string(val)
            .expect("serde_json::to_string must never panic");

        Json { json, _phantom: PhantomData }
    }

    pub fn parse(&self) -> Result<T, serde_json::Error> {
        serde_json::from_str(&self.json)
    }

    pub fn from_string(json: String) -> Self {
        Json { json, _phantom: PhantomData }
    }

    pub fn into_string(self) -> String {
        self.json
    }
}

impl<T: DeserializeOwned + Serialize> From<&T> for Json<T> {
    fn from(value: &T) -> Self {
        Self::new(value)
    }
}
