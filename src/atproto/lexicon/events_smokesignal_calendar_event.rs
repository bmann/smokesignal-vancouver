use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const NSID: &str = "events.smokesignal.calendar.event";

/// Complete response from the API including URI, CID, and value
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EventResponse {
    pub uri: String,
    pub cid: String,
    pub value: Event,
}

/// Main event structure
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "$type")]
pub enum Event {
    #[serde(rename = "events.smokesignal.calendar.event")]
    Current {
        name: String,

        #[serde(rename = "text", skip_serializing_if = "Option::is_none")]
        text: Option<String>,

        #[serde(rename = "startsAt", skip_serializing_if = "Option::is_none")]
        starts_at: Option<DateTime<Utc>>,

        #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
        created_at: Option<DateTime<Utc>>,

        #[serde(flatten)]
        extra: HashMap<String, serde_json::Value>,
    },
}

/// Location types (physical or virtual)
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "$type")]
pub enum Location {
    #[serde(rename = "events.smokesignal.calendar.location#place")]
    Place(PlaceLocation),

    #[serde(rename = "events.smokesignal.calendar.location#virtual")]
    Virtual(VirtualLocation),
}

/// Physical location information
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlaceLocation {
    /// Name of the place
    pub name: String,

    /// State or province
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,

    /// Street address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub street: Option<String>,

    /// Country code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,

    /// City or town
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locality: Option<String>,

    /// ZIP or postal code
    #[serde(rename = "postalCode", skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,
}

/// Virtual location information
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VirtualLocation {
    /// URL for the virtual location
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// Name of the virtual location
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn test_deserialize_event_response() -> Result<()> {
        let json = r#"{
            "uri": "at://did:plc:cbkjy5n7bk3ax2wplmtjofq2/events.smokesignal.calendar.event/3lklhropvnc2s",
            "cid": "bafyreia254omudtqvpzkxfd2wycaisatxhs3yhaorf2yd5dfql4rgt3qim",
            "value": {
                "mode": "events.smokesignal.calendar.event#inperson",
                "name": "Pigeons Playing Ping Pong @ Neptune Theatre",
                "text": "Pigeons Playing Ping Pong @ Neptune Theatre",
                "$type": "events.smokesignal.calendar.event",
                "endsAt": "2025-03-22T06:00:00.000Z",
                "status": "events.smokesignal.calendar.event#scheduled",
                "location": [
                    {
                        "name": "Neptune Theatre",
                        "$type": "events.smokesignal.calendar.location#place",
                        "region": "WA",
                        "street": "1303 NE 45th St",
                        "country": "US",
                        "locality": "Seattle",
                        "postalCode": "98105"
                    },
                    {
                        "url": "https://go.seated.com/tour-events/698bd09f-ab7c-49d2-82f2-aa4c8384fb26",
                        "name": "Tickets",
                        "$type": "events.smokesignal.calendar.location#virtual"
                    }
                ],
                "startsAt": "2025-03-22T03:00:00.000Z",
                "createdAt": "2025-03-17T15:28:05.972Z"
            }
        }"#;

        let event_response: EventResponse = serde_json::from_str(json)?;

        // Verify basic fields
        assert_eq!(
            event_response.uri,
            "at://did:plc:cbkjy5n7bk3ax2wplmtjofq2/events.smokesignal.calendar.event/3lklhropvnc2s"
        );
        assert_eq!(
            event_response.cid,
            "bafyreia254omudtqvpzkxfd2wycaisatxhs3yhaorf2yd5dfql4rgt3qim"
        );

        let Event::Current {
            name,
            text,
            starts_at,
            created_at,
            ..
        } = &event_response.value;

        assert_eq!(name, "Pigeons Playing Ping Pong @ Neptune Theatre");
        assert!(text
            .as_ref()
            .is_some_and(|value| value == "Pigeons Playing Ping Pong @ Neptune Theatre"));

        // Verify datetime fields are present and correctly parsed
        assert!(starts_at.is_some(), "Expected starts_at to be present");
        assert!(created_at.is_some(), "Expected created_at to be present");

        if let Some(start_time) = starts_at {
            assert_eq!(start_time.to_rfc3339(), "2025-03-22T03:00:00+00:00");
        }

        if let Some(create_time) = created_at {
            assert_eq!(create_time.to_rfc3339(), "2025-03-17T15:28:05.972+00:00");
        }

        Ok(())
    }
}
