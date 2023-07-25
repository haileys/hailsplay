use std::{error::Error, borrow::Cow};
use std::fmt::Debug;

use chrono::{DateTime, Utc};
use sqlx::{Type, Sqlite, sqlite::SqliteArgumentValue, encode::IsNull};
use toml::value::Time;

#[derive(Debug, Clone)]
pub struct Timestamp(pub DateTime<Utc>);

impl TryFrom<String> for Timestamp {
    type Error = DecodeError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let datetime = DateTime::parse_from_rfc3339(&value)?;
        Ok(Timestamp(datetime.with_timezone(&Utc)))
    }
}

impl Type<Sqlite> for Timestamp {
    fn type_info() -> <Sqlite as sqlx::Database>::TypeInfo {
        <String as Type<Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, Sqlite> for Timestamp {
    fn encode_by_ref(&self, args: &mut Vec<SqliteArgumentValue<'q>>) -> IsNull {
        let text = self.0.to_rfc3339();
        args.push(SqliteArgumentValue::Text(Cow::Owned(text)));
        IsNull::No
    }   
}

#[derive(Debug, Clone)]
pub struct Url(pub url::Url);

impl TryFrom<String> for Url {
    type Error = DecodeError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let url = url::Url::parse(&value)?;
        Ok(Url(url))
    }
}

impl Type<Sqlite> for Url {
    fn type_info() -> <Sqlite as sqlx::Database>::TypeInfo {
        <String as Type<Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, Sqlite> for Url {
    fn encode_by_ref(&self, args: &mut Vec<SqliteArgumentValue<'q>>) -> IsNull {
        let text = self.0.to_string();
        args.push(SqliteArgumentValue::Text(Cow::Owned(text)));
        IsNull::No
    }   
}

pub struct DecodeError(Box<dyn Error + Sync + Send>);

impl<E: Error + Sync + Send + 'static> From<E> for DecodeError {
    fn from(err: E) -> Self {
        DecodeError(err.into())
    }
}

impl From<DecodeError> for sqlx::Error {
    fn from(err: DecodeError) -> Self {
        sqlx::Error::Decode(err.0)
    }
}
