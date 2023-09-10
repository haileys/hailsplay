use std::fmt::Debug;
use std::marker::PhantomData;
use std::str::FromStr;

use chrono::{DateTime, Utc, FixedOffset};
use derive_more::{From, Into, Deref, DerefMut};
use diesel::backend::Backend;
use diesel::deserialize::{FromSqlRow, FromSql, self};
use diesel::sql_types::{Text as SqlText};
use diesel::{prelude::*, serialize};
use diesel::serialize::{ToSql, IsNull};
use diesel::sqlite::Sqlite;
use diesel::AsExpression;
use serde::Serialize;
use serde::de::DeserializeOwned;

#[derive(Debug, Clone, FromSqlRow, From, Into)]
#[diesel(sql_type = SqlText)]
pub struct Url(pub url::Url);

impl Expression for Url {
    type SqlType = SqlText;
}

impl FromSql<SqlText, Sqlite> for Url {
    fn from_sql(value: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let text = <String as FromSql<SqlText, Sqlite>>::from_sql(value)?;
        Ok(Url(text.parse()?))
    }
}

impl ToSql<SqlText, Sqlite> for Url {
    fn to_sql<'b>(&'b self, out: &mut serialize::Output<'b, '_, Sqlite>) -> serialize::Result {
        let text: String = self.0.to_string();
        out.set_value(text);
        Ok(IsNull::No)
    }
}

#[derive(Debug, Clone, From, Into, FromSqlRow)]
pub struct TimestampUtc(pub DateTime<Utc>);

impl FromSql<SqlText, Sqlite> for TimestampUtc {
    fn from_sql(value: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let text = <String as FromSql<SqlText, Sqlite>>::from_sql(value)?;
        let time = DateTime::<FixedOffset>::parse_from_rfc3339(&text)?;
        let utc = time.with_timezone(&Utc);
        Ok(TimestampUtc(utc))
    }
}

impl ToSql<SqlText, Sqlite> for TimestampUtc {
    fn to_sql<'b>(&'b self, out: &mut serialize::Output<'b, '_, Sqlite>) -> serialize::Result {
        let text: String = self.0.to_rfc3339();
        out.set_value(text);
        Ok(IsNull::No)
    }
}

impl Expression for TimestampUtc {
    type SqlType = SqlText;
}

#[derive(Debug, Clone, FromSqlRow, Deref, DerefMut, From, Into)]
pub struct Mime(pub mime::Mime);

impl Expression for Mime {
    type SqlType = SqlText;
}

impl FromSql<SqlText, Sqlite> for Mime {
    fn from_sql(value: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let text = <String as FromSql<SqlText, Sqlite>>::from_sql(value)?;
        Ok(Mime(text.parse()?))
    }
}

impl ToSql<SqlText, Sqlite> for Mime {
    fn to_sql<'b>(&'b self, out: &mut serialize::Output<'b, '_, Sqlite>) -> serialize::Result {
        let text: String = self.0.to_string();
        out.set_value(text);
        Ok(IsNull::No)
    }
}

#[derive(Debug, Clone, FromSqlRow)]
#[diesel(sql_type = SqlText)]
pub struct Json<T: DeserializeOwned + Serialize + Debug> {
    json: String,
    _phantom: PhantomData<T>,
}

impl<T: DeserializeOwned + Serialize + Debug> Expression for Json<T> {
    type SqlType = SqlText;
}

impl<T: DeserializeOwned + Serialize + Debug> FromSql<SqlText, Sqlite> for Json<T> {
    fn from_sql(value: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let text = <String as FromSql<SqlText, Sqlite>>::from_sql(value)?;
        Ok(Json::from_string(text))
    }
}

impl<T: DeserializeOwned + Serialize + Debug> ToSql<SqlText, Sqlite> for Json<T> {
    fn to_sql<'b>(&'b self, out: &mut serialize::Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(&self.json);
        Ok(IsNull::No)
    }
}
impl<T: DeserializeOwned + Serialize + Debug> Json<T> {
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

impl<T: DeserializeOwned + Serialize + Debug> From<&T> for Json<T> {
    fn from(value: &T) -> Self {
        Self::new(value)
    }
}
