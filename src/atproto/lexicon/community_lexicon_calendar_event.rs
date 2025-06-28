use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::atproto::lexicon::community::lexicon::location::{Address, Fsq, Geo, Hthree};
use crate::atproto::{
    datetime::format as datetime_format, datetime::optional_format as optional_datetime_format,
};

pub const NSID: &str = "community.lexicon.calendar.event";

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
pub enum Status {
    #[default]
    #[serde(rename = "community.lexicon.calendar.event#scheduled")]
    Scheduled,

    #[serde(rename = "community.lexicon.calendar.event#rescheduled")]
    Rescheduled,

    #[serde(rename = "community.lexicon.calendar.event#cancelled")]
    Cancelled,

    #[serde(rename = "community.lexicon.calendar.event#postponed")]
    Postponed,

    #[serde(rename = "community.lexicon.calendar.event#planned")]
    Planned,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
pub enum Mode {
    #[default]
    #[serde(rename = "community.lexicon.calendar.event#inperson")]
    InPerson,

    #[serde(rename = "community.lexicon.calendar.event#virtual")]
    Virtual,

    #[serde(rename = "community.lexicon.calendar.event#hybrid")]
    Hybrid,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "$type")]
pub enum NamedUri {
    #[serde(rename = "community.lexicon.calendar.event#uri")]
    Current {
        uri: String,

        #[serde(skip_serializing_if = "Option::is_none", default)]
        name: Option<String>,
    },
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum EventLocation {
    Uri(NamedUri),

    Address(Address),

    Geo(Geo),

    Fsq(Fsq),

    Hthree(Hthree),
}

pub type EventLocations = Vec<EventLocation>;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "$type")]
pub enum EventLink {
    #[serde(rename = "community.lexicon.calendar.event#uri")]
    Current {
        uri: String,

        #[serde(skip_serializing_if = "Option::is_none", default)]
        name: Option<String>,
    },
}

pub type EventLinks = Vec<EventLink>;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "$type")]
pub enum Event {
    #[serde(rename = "community.lexicon.calendar.event")]
    Current {
        name: String,

        description: String,

        #[serde(rename = "createdAt", with = "datetime_format")]
        created_at: DateTime<Utc>,

        #[serde(
            rename = "startsAt",
            skip_serializing_if = "Option::is_none",
            default,
            with = "optional_datetime_format"
        )]
        starts_at: Option<DateTime<Utc>>,

        #[serde(
            rename = "endsAt",
            skip_serializing_if = "Option::is_none",
            default,
            with = "optional_datetime_format"
        )]
        ends_at: Option<DateTime<Utc>>,

        #[serde(rename = "mode", skip_serializing_if = "Option::is_none", default)]
        mode: Option<Mode>,

        #[serde(rename = "status", skip_serializing_if = "Option::is_none", default)]
        status: Option<Status>,

        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        locations: EventLocations,

        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        uris: EventLinks,

        // This is a catch-all for any elements that are not implemented by
        // Smoke Signal. This allows them to be maintained and indexed, for
        // potential use later without having to re-index content.
        #[serde(flatten)]
        extra: HashMap<String, serde_json::Value>,
        // TODO: Propose the following lexicon changes:
        // * root, StrongRef - Allows a root object to be set for the purpose of creating a heirarchy.
        // * parent, StrongRef - Allows a parent object to be set for the purpose of creating a heirarchy.
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn location_record() -> Result<()> {
        let test_json = r#"{"$type":"community.lexicon.calendar.event#uri","uri":"https://smokesignal.events/","name":"Smoke Signal"}"#;

        {
            // Serialize bare
            assert_eq!(
                serde_json::to_string(&NamedUri::Current {
                    uri: "https://smokesignal.events/".to_string(),
                    name: Some("Smoke Signal".to_string())
                })
                .unwrap(),
                test_json
            );
        }

        {
            // Serialize wrapped
            assert_eq!(
                serde_json::to_string(&EventLocation::Uri(NamedUri::Current {
                    uri: "https://smokesignal.events/".to_string(),
                    name: Some("Smoke Signal".to_string())
                }))
                .unwrap(),
                test_json
            );
        }

        {
            // Serialize plural
            let locations: EventLocations = vec![EventLocation::Uri(NamedUri::Current {
                uri: "https://smokesignal.events/".to_string(),
                name: Some("Smoke Signal".to_string()),
            })];
            assert_eq!(
                serde_json::to_string(&locations).unwrap(),
                format!("[{}]", test_json)
            );
        }

        {
            // Deserialize bare
            let deserialized: NamedUri = serde_json::from_str(test_json).unwrap();

            assert_eq!(
                NamedUri::Current {
                    uri: "https://smokesignal.events/".to_string(),
                    name: Some("Smoke Signal".to_string())
                },
                deserialized
            );
        }

        {
            // Deserialize wrapped
            let deserialized: EventLocation = serde_json::from_str(test_json).unwrap();

            assert_eq!(
                EventLocation::Uri(NamedUri::Current {
                    uri: "https://smokesignal.events/".to_string(),
                    name: Some("Smoke Signal".to_string())
                }),
                deserialized
            );
        }

        {
            // Deserialize plural
            let deserialized: EventLocations =
                serde_json::from_str(&format!("[{}]", test_json)).unwrap();

            let locations: EventLocations = vec![EventLocation::Uri(NamedUri::Current {
                uri: "https://smokesignal.events/".to_string(),
                name: Some("Smoke Signal".to_string()),
            })];
            assert_eq!(locations, deserialized);
        }

        Ok(())
    }
}
