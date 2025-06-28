use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::atproto::datetime::optional_format as optional_datetime_format;
use crate::atproto::lexicon::com::atproto::repo::StrongRef;

pub const NSID: &str = "events.smokesignal.calendar.rsvp";

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
pub enum RsvpStatus {
    #[default]
    #[serde(rename = "events.smokesignal.calendar.rsvp#going")]
    Going,

    #[serde(rename = "events.smokesignal.calendar.rsvp#interested")]
    Interested,

    #[serde(rename = "events.smokesignal.calendar.rsvp#notgoing")]
    NotGoing,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "$type")]
pub enum Rsvp {
    #[serde(rename = "events.smokesignal.calendar.rsvp")]
    Current {
        subject: StrongRef,

        status: RsvpStatus,

        #[serde(
            rename = "createdAt",
            with = "optional_datetime_format",
            skip_serializing_if = "Option::is_none",
            default
        )]
        created_at: Option<DateTime<Utc>>,
    },
}
