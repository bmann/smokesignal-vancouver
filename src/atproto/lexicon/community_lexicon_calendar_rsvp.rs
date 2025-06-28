use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::atproto::datetime::format as datetime_format;
use crate::atproto::lexicon::com::atproto::repo::StrongRef;

pub const NSID: &str = "community.lexicon.calendar.rsvp";

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
pub enum RsvpStatus {
    #[default]
    #[serde(rename = "community.lexicon.calendar.rsvp#going")]
    Going,

    #[serde(rename = "community.lexicon.calendar.rsvp#interested")]
    Interested,

    #[serde(rename = "community.lexicon.calendar.rsvp#notgoing")]
    NotGoing,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "$type")]
pub enum Rsvp {
    #[serde(rename = "community.lexicon.calendar.rsvp")]
    Current {
        subject: StrongRef,

        status: RsvpStatus,

        #[serde(rename = "createdAt", with = "datetime_format")]
        created_at: DateTime<Utc>,
    },
}
