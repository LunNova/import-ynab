use crate::prelude::*;
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::de;
use std::fmt;

struct DateTimeFromCustomFormatVisitor;

pub fn deserialize<'de, D>(d: D) -> Result<UtcDateTime, D::Error>
where
    D: de::Deserializer<'de>,
{
    d.deserialize_str(DateTimeFromCustomFormatVisitor)
}

impl<'de> de::Visitor<'de> for DateTimeFromCustomFormatVisitor {
    type Value = UtcDateTime;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a datetime string")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match chrono::DateTime::parse_from_rfc3339(value) {
            Ok(date) => Ok(date.into()),
            Err(_e) => match NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S") {
                Ok(ndt) => Ok(DateTime::from_utc(ndt, Utc)),
                Err(e) => Err(E::custom(format!("Parse error {} for {}", e, value))),
            },
        }
    }
}
