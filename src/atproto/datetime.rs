pub mod format {
    use chrono::{DateTime, SecondsFormat, Utc};
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = date.to_rfc3339_opts(SecondsFormat::Millis, true);
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let date_value = String::deserialize(deserializer)?;
        DateTime::parse_from_rfc3339(&date_value)
            .map(|v| v.with_timezone(&Utc))
            .map_err(serde::de::Error::custom)
    }
}

pub mod optional_format {
    use chrono::{DateTime, SecondsFormat, Utc};
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(date: &Option<DateTime<Utc>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if date.is_none() {
            return serializer.serialize_none();
        }
        let s = date.unwrap().to_rfc3339_opts(SecondsFormat::Millis, true);
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let maybe_date_value: Option<String> = Option::deserialize(deserializer)?;
        if maybe_date_value.is_none() {
            return Ok(None);
        }
        let date_value = maybe_date_value.unwrap();
        DateTime::parse_from_rfc3339(&date_value)
            .map(|v| v.with_timezone(&Utc))
            .map_err(serde::de::Error::custom)
            .map(Some)
    }
}
