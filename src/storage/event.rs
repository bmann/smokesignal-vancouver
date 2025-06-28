use std::borrow::Cow;
use std::collections::HashMap;

use anyhow::Result;
use chrono::Utc;
use serde_json::json;
use sqlx::{Postgres, QueryBuilder};

use crate::atproto::lexicon::community::lexicon::calendar::event::Event as EventLexicon;
use crate::atproto::lexicon::community::lexicon::calendar::rsvp::{
    Rsvp as RsvpLexicon, RsvpStatus as RsvpStatusLexicon,
};

use super::errors::StorageError;
use super::StoragePool;
use model::{Event, EventWithRole, Rsvp};

pub mod model {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};
    use sqlx::FromRow;

    #[derive(Clone, FromRow, Deserialize, Serialize, Debug)]
    pub struct Event {
        pub aturi: String,
        pub cid: String,

        pub did: String,
        pub lexicon: String,

        pub record: sqlx::types::Json<serde_json::Value>,

        pub name: String,

        pub updated_at: Option<DateTime<Utc>>,
    }

    #[derive(Clone, FromRow, Debug, Serialize)]
    pub struct EventWithRole {
        #[sqlx(flatten)]
        pub event: Event,

        pub role: String,
        // pub event_handle: String,
    }

    #[derive(Clone, FromRow, Deserialize, Serialize, Debug)]
    pub struct Rsvp {
        pub aturi: String,
        pub cid: String,

        pub did: String,
        pub lexicon: String,

        pub record: sqlx::types::Json<serde_json::Value>,

        pub event_aturi: String,
        pub event_cid: String,
        pub status: String,

        pub updated_at: Option<DateTime<Utc>>,
    }
}

pub async fn event_insert(
    pool: &StoragePool,
    aturi: &str,
    cid: &str,
    did: &str,
    lexicon: &str,
    record: &EventLexicon,
) -> Result<(), StorageError> {
    // Extract name from the record
    let name = match record {
        EventLexicon::Current { name, .. } => name,
    };

    // Call the new function with extracted values
    event_insert_with_metadata(pool, aturi, cid, did, lexicon, record, name).await
}

pub async fn event_insert_with_metadata<T: serde::Serialize>(
    pool: &StoragePool,
    aturi: &str,
    cid: &str,
    did: &str,
    lexicon: &str,
    record: &T,
    name: &str,
) -> Result<(), StorageError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(StorageError::CannotBeginDatabaseTransaction)?;

    let now = Utc::now();

    sqlx::query("INSERT INTO events (aturi, cid, did, lexicon, record, name, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)")
        .bind(aturi)
        .bind(cid)
        .bind(did)
        .bind(lexicon)
        .bind(json!(record))
        .bind(name)
        .bind(now)
        .execute(tx.as_mut())
        .await
        .map_err(StorageError::UnableToExecuteQuery)?;

    tx.commit()
        .await
        .map_err(StorageError::CannotCommitDatabaseTransaction)
}

pub struct RsvpInsertParams<'a, T: serde::Serialize> {
    pub aturi: &'a str,
    pub cid: &'a str,
    pub did: &'a str,
    pub lexicon: &'a str,
    pub record: &'a T,
    pub event_aturi: &'a str,
    pub event_cid: &'a str,
    pub status: &'a str,
}

pub async fn rsvp_insert_with_metadata<T: serde::Serialize>(
    pool: &StoragePool,
    params: RsvpInsertParams<'_, T>,
) -> Result<(), StorageError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(StorageError::CannotBeginDatabaseTransaction)?;

    let now = Utc::now();

    sqlx::query("INSERT INTO rsvps (aturi, cid, did, lexicon, record, event_aturi, event_cid, status, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) ON CONFLICT (aturi) DO UPDATE SET record = $5, cid = $2, status = $8, updated_at = $9")
            .bind(params.aturi)
            .bind(params.cid)
            .bind(params.did)
            .bind(params.lexicon)
            .bind(json!(params.record))
            .bind(params.event_aturi)
            .bind(params.event_cid)
            .bind(params.status)
            .bind(now)
            .execute(tx.as_mut())
            .await
            .map_err(StorageError::UnableToExecuteQuery)?;

    tx.commit()
        .await
        .map_err(StorageError::CannotCommitDatabaseTransaction)
}

pub async fn rsvp_insert(
    pool: &StoragePool,
    aturi: &str,
    cid: &str,
    did: &str,
    lexicon: &str,
    record: &RsvpLexicon,
) -> Result<(), StorageError> {
    // Extract the metadata from the record
    let (event_aturi, event_cid, status) = match record {
        RsvpLexicon::Current {
            subject, status, ..
        } => {
            let event_aturi = subject.uri.clone();
            let event_cid = subject.cid.clone();
            let status = match status {
                RsvpStatusLexicon::Going => "going",
                RsvpStatusLexicon::Interested => "interested",
                RsvpStatusLexicon::NotGoing => "notgoing",
            };
            (event_aturi, event_cid, status)
        }
    };

    // Call the generic function with extracted values
    rsvp_insert_with_metadata(
        pool,
        RsvpInsertParams {
            aturi,
            cid,
            did,
            lexicon,
            record,
            event_aturi: &event_aturi,
            event_cid: &event_cid,
            status,
        },
    )
    .await
}

// Helper function to extract event information based on lexicon type
// Helper function to format address information into a readable string
pub fn format_address(
    address: &crate::atproto::lexicon::community::lexicon::location::Address,
) -> String {
    match address {
        crate::atproto::lexicon::community::lexicon::location::Address::Current {
            country,
            postal_code,
            region,
            locality,
            street,
            name,
        } => {
            let mut parts = Vec::new();

            // Add parts in specified order, omitting empty values
            if let Some(name_val) = name {
                if !name_val.trim().is_empty() {
                    parts.push(name_val.clone());
                }
            }

            if let Some(street_val) = street {
                if !street_val.trim().is_empty() {
                    parts.push(street_val.clone());
                }
            }

            if let Some(locality_val) = locality {
                if !locality_val.trim().is_empty() {
                    parts.push(locality_val.clone());
                }
            }

            if let Some(region_val) = region {
                if !region_val.trim().is_empty() {
                    parts.push(region_val.clone());
                }
            }

            if let Some(postal_val) = postal_code {
                if !postal_val.trim().is_empty() {
                    parts.push(postal_val.clone());
                }
            }

            // Country is required so no need to check if it's empty
            parts.push(country.clone());

            // Join parts with commas
            parts.join(", ")
        }
    }
}

pub fn extract_event_details(event: &Event) -> EventDetails {
    use crate::atproto::lexicon::{
        community::lexicon::calendar::event::{Event as CommunityEvent, Mode, Status},
        events::smokesignal::calendar::event::Event as SmokeSignalEvent,
    };

    // Try to parse the record based on the lexicon
    match event.lexicon.as_str() {
        "community.lexicon.calendar.event" => {
            if let Ok(community_event) =
                serde_json::from_value::<CommunityEvent>(event.record.0.clone())
            {
                match community_event {
                    CommunityEvent::Current {
                        name,
                        description,
                        created_at,
                        starts_at,
                        ends_at,
                        mode,
                        status,
                        locations,
                        uris,
                        ..
                    } => EventDetails {
                        name: Cow::Owned(name.clone()),
                        description: Cow::Owned(description.clone()),
                        created_at: Some(created_at),
                        starts_at,
                        ends_at,
                        mode: mode.map(|m| match m {
                            Mode::InPerson => {
                                Cow::Borrowed("community.lexicon.calendar.event#inperson")
                            }
                            Mode::Virtual => {
                                Cow::Borrowed("community.lexicon.calendar.event#virtual")
                            }
                            Mode::Hybrid => {
                                Cow::Borrowed("community.lexicon.calendar.event#hybrid")
                            }
                        }),
                        status: status.map(|s| match s {
                            Status::Scheduled => {
                                Cow::Borrowed("community.lexicon.calendar.event#scheduled")
                            }
                            Status::Rescheduled => {
                                Cow::Borrowed("community.lexicon.calendar.event#rescheduled")
                            }
                            Status::Cancelled => {
                                Cow::Borrowed("community.lexicon.calendar.event#cancelled")
                            }
                            Status::Postponed => {
                                Cow::Borrowed("community.lexicon.calendar.event#postponed")
                            }
                            Status::Planned => {
                                Cow::Borrowed("community.lexicon.calendar.event#planned")
                            }
                        }),
                        locations,
                        uris,
                    },
                }
            } else {
                // Fallback to the event's direct name if parsing fails
                EventDetails {
                    name: Cow::Owned(event.name.clone()),
                    description: Cow::Borrowed(""),
                    created_at: None,
                    starts_at: None,
                    ends_at: None,
                    mode: None,
                    status: None,
                    locations: vec![],
                    uris: vec![],
                }
            }
        }
        "events.smokesignal.calendar.event" => {
            if let Ok(ss_event) = serde_json::from_value::<SmokeSignalEvent>(event.record.0.clone())
            {
                match ss_event {
                    SmokeSignalEvent::Current {
                        name,
                        text,
                        created_at,
                        starts_at,
                        extra,
                        ..
                    } => {
                        // Extract additional fields from extra map
                        let ends_at = extra
                            .get("endsAt")
                            .and_then(|v| v.as_str())
                            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                            .map(|dt| dt.with_timezone(&chrono::Utc));

                        let mode = extra
                            .get("mode")
                            .and_then(|v| v.as_str().map(ToString::to_string));
                        let status = extra
                            .get("status")
                            .and_then(|v| v.as_str().map(ToString::to_string));

                        // Convert locations to the same format used by community.lexicon.calendar.event
                        // Process locations from extra data if available
                        let locations = Vec::new();

                        // Extract links from location data
                        let mut uris = Vec::new();

                        // Check for virtual locations in the location array
                        if let Some(location_value) = extra.get("location") {
                            if let Some(location_array) = location_value.as_array() {
                                for loc in location_array {
                                    if let Some(loc_type) = loc.get("$type") {
                                        if let Some(loc_type_str) = loc_type.as_str() {
                                            // Handle virtual locations as links
                                            if loc_type_str
                                                == "events.smokesignal.calendar.location#virtual"
                                            {
                                                if let (Some(url), Some(name)) = (
                                                    loc.get("url").and_then(|u| u.as_str()),
                                                    loc.get("name").and_then(|n| n.as_str()),
                                                ) {
                                                    uris.push(crate::atproto::lexicon::community::lexicon::calendar::event::EventLink::Current {
                                                        uri: url.to_string(),
                                                        name: Some(name.to_string()),
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Also check for any additional URIs in the extra map
                        if let Some(links_value) = extra.get("links") {
                            if let Some(links_array) = links_value.as_array() {
                                for link in links_array {
                                    if let (Some(uri), Some(name)) = (
                                        link.get("uri").and_then(|u| u.as_str()),
                                        link.get("name").and_then(|n| n.as_str()),
                                    ) {
                                        uris.push(crate::atproto::lexicon::community::lexicon::calendar::event::EventLink::Current {
                                            uri: uri.to_string(),
                                            name: Some(name.to_string()),
                                        });
                                    } else if let Some(uri) =
                                        link.get("uri").and_then(|u| u.as_str())
                                    {
                                        uris.push(crate::atproto::lexicon::community::lexicon::calendar::event::EventLink::Current {
                                            uri: uri.to_string(),
                                            name: None,
                                        });
                                    }
                                }
                            }
                        }

                        EventDetails {
                            name: Cow::Owned(name.clone()),
                            description: Cow::Owned(text.clone().unwrap_or_default()),
                            created_at,
                            starts_at,
                            ends_at: ends_at.map(Some).unwrap_or(None),
                            mode: mode.map(Cow::Owned),
                            status: status.map(Cow::Owned),
                            locations,
                            uris,
                        }
                    }
                }
            } else {
                // Fallback to the event's direct name if parsing fails
                EventDetails {
                    name: Cow::Owned(event.name.clone()),
                    description: Cow::Borrowed(""),
                    created_at: None,
                    starts_at: None,
                    ends_at: None,
                    mode: None,
                    status: None,
                    locations: vec![],
                    uris: vec![],
                }
            }
        }
        _ => {
            // Unknown event type - use the stored name
            EventDetails {
                name: Cow::Owned(event.name.clone()),
                description: Cow::Borrowed(""),
                created_at: None,
                starts_at: None,
                ends_at: None,
                mode: None,
                status: None,
                locations: vec![],
                uris: vec![],
            }
        }
    }
}

// Structure to hold extracted event details regardless of source format
#[derive(Debug, Clone)]
pub struct EventDetails {
    pub name: Cow<'static, str>,
    pub description: Cow<'static, str>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub starts_at: Option<chrono::DateTime<chrono::Utc>>,
    pub ends_at: Option<chrono::DateTime<chrono::Utc>>,
    pub mode: Option<Cow<'static, str>>,
    pub status: Option<Cow<'static, str>>,
    pub locations: Vec<crate::atproto::lexicon::community::lexicon::calendar::event::EventLocation>,
    pub uris: Vec<crate::atproto::lexicon::community::lexicon::calendar::event::EventLink>,
}

pub async fn event_get(pool: &StoragePool, aturi: &str) -> Result<Event, StorageError> {
    // Validate aturi is not empty
    if aturi.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "Event URI cannot be empty".into(),
        )));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(StorageError::CannotBeginDatabaseTransaction)?;

    let record = sqlx::query_as::<_, Event>("SELECT * FROM events WHERE aturi = $1")
        .bind(aturi)
        .fetch_one(tx.as_mut())
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => StorageError::RowNotFound("event".to_string(), err),
            other => StorageError::UnableToExecuteQuery(other),
        })?;

    tx.commit()
        .await
        .map_err(StorageError::CannotCommitDatabaseTransaction)?;

    Ok(record)
}

pub async fn event_exists(pool: &StoragePool, aturi: &str) -> Result<bool, StorageError> {
    // Validate aturi is not empty
    if aturi.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "Event URI cannot be empty".into(),
        )));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(StorageError::CannotBeginDatabaseTransaction)?;

    let total_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM events WHERE aturi = $1")
        .bind(aturi)
        .fetch_one(tx.as_mut())
        .await
        .map_err(StorageError::UnableToExecuteQuery)?;

    tx.commit()
        .await
        .map_err(StorageError::CannotCommitDatabaseTransaction)?;

    Ok(total_count > 0)
}

pub async fn event_get_cid(
    pool: &StoragePool,
    aturi: &str,
) -> Result<Option<String>, StorageError> {
    // Validate aturi is not empty
    if aturi.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "Event URI cannot be empty".into(),
        )));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(StorageError::CannotBeginDatabaseTransaction)?;

    let record = sqlx::query_scalar::<_, String>("SELECT cid FROM events WHERE aturi = $1")
        .bind(aturi)
        .fetch_optional(tx.as_mut())
        .await
        .map_err(StorageError::UnableToExecuteQuery)?;

    tx.commit()
        .await
        .map_err(StorageError::CannotCommitDatabaseTransaction)?;

    Ok(record)
}

pub async fn event_list_did_recently_updated(
    pool: &StoragePool,
    did: &str,
    page: i64,
    page_size: i64,
) -> Result<Vec<EventWithRole>, StorageError> {
    // Validate did is not empty
    if did.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "DID cannot be empty".into(),
        )));
    }

    // Validate page and page_size are positive
    if page < 1 || page_size < 1 {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "Page and page size must be positive".into(),
        )));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(StorageError::CannotBeginDatabaseTransaction)?;

    let offset = (page - 1) * page_size;

    let events_query = r"SELECT
    events.*,
    'organizer' as role
FROM
    events
WHERE
    events.did = $1
ORDER BY
    events.updated_at DESC,
    events.aturi ASC
LIMIT
$2
OFFSET
$3
";

    let event_roles = sqlx::query_as::<_, EventWithRole>(events_query)
        .bind(did)
        .bind(page_size + 1)
        .bind(offset)
        .fetch_all(tx.as_mut())
        .await
        .map_err(StorageError::UnableToExecuteQuery)?;

    tx.commit()
        .await
        .map_err(StorageError::CannotCommitDatabaseTransaction)?;

    Ok(event_roles)
}

pub async fn event_list_recently_updated(
    pool: &StoragePool,
    page: i64,
    page_size: i64,
) -> Result<Vec<EventWithRole>, StorageError> {
    // Validate page and page_size are positive
    if page < 1 || page_size < 1 {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "Page and page size must be positive".into(),
        )));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(StorageError::CannotBeginDatabaseTransaction)?;

    let offset = (page - 1) * page_size;

    let events_query = r"SELECT
        events.*,
        'organizer' as role
    FROM
        events
    ORDER BY
        events.updated_at DESC,
        events.aturi ASC
    LIMIT $1
    OFFSET $2";

    let event_roles = sqlx::query_as::<_, EventWithRole>(events_query)
        .bind(page_size + 1)
        .bind(offset)
        .fetch_all(tx.as_mut())
        .await
        .map_err(StorageError::UnableToExecuteQuery)?;

    tx.commit()
        .await
        .map_err(StorageError::CannotCommitDatabaseTransaction)?;

    Ok(event_roles)
}

pub async fn get_event_rsvps(
    pool: &StoragePool,
    event_aturi: &str,
    status: Option<&str>,
) -> Result<Vec<(String, String)>, StorageError> {
    // Validate event_aturi is not empty
    if event_aturi.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "Event URI cannot be empty".into(),
        )));
    }

    // If status is provided, validate it's not empty
    if let Some(status_val) = status {
        if status_val.trim().is_empty() {
            return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
                "Status cannot be empty".into(),
            )));
        }
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(StorageError::CannotBeginDatabaseTransaction)?;

    let query = if status.is_some() {
        "SELECT did, status FROM rsvps WHERE event_aturi = $1 AND status = $2"
    } else {
        "SELECT did, status FROM rsvps WHERE event_aturi = $1"
    };

    let rsvps = if let Some(status_value) = status {
        sqlx::query_as::<_, (String, String)>(query)
            .bind(event_aturi)
            .bind(status_value)
            .fetch_all(tx.as_mut())
            .await
    } else {
        sqlx::query_as::<_, (String, String)>(query)
            .bind(event_aturi)
            .fetch_all(tx.as_mut())
            .await
    }
    .map_err(StorageError::UnableToExecuteQuery)?;

    tx.commit()
        .await
        .map_err(StorageError::CannotCommitDatabaseTransaction)?;

    Ok(rsvps)
}

pub async fn get_user_rsvp(
    pool: &StoragePool,
    event_aturi: &str,
    did: &str,
) -> Result<Option<String>, StorageError> {
    // Validate event_aturi is not empty
    if event_aturi.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "Event URI cannot be empty".into(),
        )));
    }

    // Validate did is not empty
    if did.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "DID cannot be empty".into(),
        )));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(StorageError::CannotBeginDatabaseTransaction)?;

    let status = sqlx::query_scalar::<_, String>(
        "SELECT status FROM rsvps WHERE event_aturi = $1 AND did = $2",
    )
    .bind(event_aturi)
    .bind(did)
    .fetch_optional(tx.as_mut())
    .await
    .map_err(StorageError::UnableToExecuteQuery)?;

    tx.commit()
        .await
        .map_err(StorageError::CannotCommitDatabaseTransaction)?;

    Ok(status)
}

pub async fn rsvp_get(pool: &StoragePool, aturi: &str) -> Result<Option<Rsvp>, StorageError> {
    // Validate aturi is not empty
    if aturi.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "RSVP URI cannot be empty".into(),
        )));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(StorageError::CannotBeginDatabaseTransaction)?;

    let rsvp = sqlx::query_as::<_, Rsvp>("SELECT * FROM rsvps WHERE aturi = $1")
        .bind(aturi)
        .fetch_optional(tx.as_mut())
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => StorageError::RSVPNotFound,
            other => StorageError::UnableToExecuteQuery(other),
        })?;

    tx.commit()
        .await
        .map_err(StorageError::CannotCommitDatabaseTransaction)?;

    Ok(rsvp)
}

pub async fn rsvp_list(
    pool: &StoragePool,
    page: i64,
    page_size: i64,
) -> Result<(i64, Vec<Rsvp>), StorageError> {
    // Validate page and page_size are positive
    if page < 1 || page_size < 1 {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "Page and page size must be positive".into(),
        )));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(StorageError::CannotBeginDatabaseTransaction)?;

    let total_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM rsvps")
        .fetch_one(tx.as_mut())
        .await
        .map_err(StorageError::UnableToExecuteQuery)?;

    let offset = (page - 1) * page_size;

    let rsvps = sqlx::query_as::<_, Rsvp>(
        r"SELECT * FROM rsvps ORDER BY rsvps.updated_at DESC LIMIT $1 OFFSET $2",
    )
    .bind(page_size + 1) // Fetch one more to know if there are more entries
    .bind(offset)
    .fetch_all(tx.as_mut())
    .await
    .map_err(StorageError::UnableToExecuteQuery)?;

    tx.commit()
        .await
        .map_err(StorageError::CannotCommitDatabaseTransaction)?;

    Ok((total_count, rsvps))
}

pub async fn event_update_with_metadata<T: serde::Serialize>(
    pool: &StoragePool,
    aturi: &str,
    cid: &str,
    record: &T,
    name: &str,
) -> Result<(), StorageError> {
    // Validate inputs
    if aturi.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "Event URI cannot be empty".into(),
        )));
    }

    if cid.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "CID cannot be empty".into(),
        )));
    }

    if name.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "Name cannot be empty".into(),
        )));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(StorageError::CannotBeginDatabaseTransaction)?;

    let now = Utc::now();

    sqlx::query(
        "UPDATE events SET cid = $1, record = $2, name = $3, updated_at = $4 WHERE aturi = $5",
    )
    .bind(cid)
    .bind(json!(record))
    .bind(name)
    .bind(now)
    .bind(aturi)
    .execute(tx.as_mut())
    .await
    .map_err(StorageError::UnableToExecuteQuery)?;

    tx.commit()
        .await
        .map_err(StorageError::CannotCommitDatabaseTransaction)
}

pub async fn count_event_rsvps(
    pool: &StoragePool,
    event_aturi: &str,
    status: &str,
) -> Result<u32, StorageError> {
    // Validate inputs
    if event_aturi.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "Event URI cannot be empty".into(),
        )));
    }

    if status.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "Status cannot be empty".into(),
        )));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(StorageError::CannotBeginDatabaseTransaction)?;

    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM rsvps WHERE event_aturi = $1 AND status = $2",
    )
    .bind(event_aturi)
    .bind(status)
    .fetch_one(tx.as_mut())
    .await
    .map_err(StorageError::UnableToExecuteQuery)?;

    tx.commit()
        .await
        .map_err(StorageError::CannotCommitDatabaseTransaction)?;

    Ok(count as u32)
}

pub async fn get_event_rsvp_counts(
    pool: &StoragePool,
    aturis: Vec<String>,
) -> Result<HashMap<(std::string::String, std::string::String), i64>, StorageError> {
    // Handle empty list case
    if aturis.is_empty() {
        return Ok(HashMap::new());
    }

    // Validate all aturis are non-empty
    for aturi in &aturis {
        if aturi.trim().is_empty() {
            return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
                "Event URI cannot be empty".into(),
            )));
        }
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(StorageError::CannotBeginDatabaseTransaction)?;

    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT event_aturi, status, COUNT(*) as count FROM rsvps WHERE event_aturi IN (",
    );
    let mut separated = query_builder.separated(", ");
    for aturi in &aturis {
        separated.push_bind(aturi);
    }
    separated.push_unseparated(") GROUP BY event_aturi, status");

    // Use build_query_as to correctly include the bindings
    let query = query_builder.build_query_as::<(String, String, i64)>();
    let values = query
        .fetch_all(tx.as_mut())
        .await
        .map_err(StorageError::UnableToExecuteQuery)?;

    tx.commit()
        .await
        .map_err(StorageError::CannotCommitDatabaseTransaction)?;

    Ok(HashMap::from_iter(values.iter().map(
        |(aturi, status, count)| ((aturi.clone(), status.clone()), *count),
    )))
}

pub async fn event_list(
    pool: &StoragePool,
    page: i64,
    page_size: i64,
) -> Result<(i64, Vec<Event>), StorageError> {
    // Validate page and page_size are positive
    if page < 1 || page_size < 1 {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "Page and page size must be positive".into(),
        )));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(StorageError::CannotBeginDatabaseTransaction)?;

    let total_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM events")
        .fetch_one(tx.as_mut())
        .await
        .map_err(StorageError::UnableToExecuteQuery)?;

    let offset = (page - 1) * page_size;

    let events = sqlx::query_as::<_, Event>(
        "SELECT * FROM events ORDER BY updated_at DESC LIMIT $1 OFFSET $2",
    )
    .bind(page_size + 1) // Fetch one more to know if there are more entries
    .bind(offset)
    .fetch_all(tx.as_mut())
    .await
    .map_err(StorageError::UnableToExecuteQuery)?;

    tx.commit()
        .await
        .map_err(StorageError::CannotCommitDatabaseTransaction)?;

    Ok((total_count, events))
}
