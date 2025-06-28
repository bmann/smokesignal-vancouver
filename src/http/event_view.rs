use std::collections::HashSet;

use ammonia::Builder;
use anyhow::Result;
use chrono_tz::Tz;
use cityhasher::HashMap;
use serde::Serialize;

use crate::http::errors::EventViewError;

use crate::{
    atproto::{
        lexicon::{
            community::lexicon::calendar::event::NSID as LexiconCommunityEventNSID,
            events::smokesignal::calendar::event::NSID as SmokeSignalEventNSID,
        },
        uri::parse_aturi,
    },
    http::utils::truncate_text,
    storage::{
        errors::StorageError,
        event::{
            count_event_rsvps, extract_event_details, get_event_rsvp_counts,
            model::{Event, EventWithRole},
        },
        handle::{handles_by_did, model::Handle},
        StoragePool,
    },
};

#[derive(Serialize, Debug, Clone)]
pub struct EventView {
    pub site_url: String,
    pub aturi: String,
    pub cid: String,
    pub repository: String,
    pub collection: String,

    pub organizer_did: String,
    pub organizer_display_name: String,

    pub starts_at_machine: Option<String>,
    pub starts_at_human: Option<String>,
    pub ends_at_machine: Option<String>,
    pub ends_at_human: Option<String>,

    pub name: String,
    pub description: Option<String>,
    pub description_short: Option<String>,

    pub count_going: u32,
    pub count_notgoing: u32,
    pub count_interested: u32,

    pub mode: Option<String>,
    pub status: Option<String>,
    pub address_display: Option<String>,
    pub links: Vec<(String, Option<String>)>, // (uri, name)
}

impl TryFrom<(Option<&Handle>, Option<&Handle>, &Event)> for EventView {
    type Error = anyhow::Error;

    fn try_from(
        (viewer, organizer, event): (Option<&Handle>, Option<&Handle>, &Event),
    ) -> Result<Self, Self::Error> {
        // Time zones are used to display date/time values from the perspective
        // of the viewer. The timezone is selected with this priority:
        // 1. If the viewer is a logged in user, use their time zone
        // 2. If the event has a starts at, use the time zone associated with it (not possible with current model)
        // 3. If the event has a ends at, use the time zone associated with it (not possible with current model)
        // 4. If the event organizer is known and has a time zone set
        // 5. UTC

        let tz = match (viewer, organizer) {
            (Some(handle), _) => handle.tz.parse::<Tz>().ok(),
            (_, Some(handle)) => handle.tz.parse::<Tz>().ok(),
            _ => None,
        }
        .unwrap_or(Tz::UTC);

        let (repository, collection, rkey) = parse_aturi(event.aturi.as_str())?;

        // We now support both community and smokesignal event formats
        if collection != LexiconCommunityEventNSID && collection != SmokeSignalEventNSID {
            return Err(EventViewError::InvalidCollection(collection).into());
        }

        let organizer_did = repository.clone();
        let organizer_display_name = organizer
            .map(|value| value.handle.clone())
            .unwrap_or_else(|| organizer_did.clone());

        // Extract event details using our new helper
        let details = extract_event_details(event);

        // Clean the name and description
        let event_name = Builder::new()
            .tags(HashSet::new())
            .clean(&details.name)
            .to_string();

        let event_description = Some(
            Builder::new()
                .tags(HashSet::new())
                .clean(&details.description)
                .to_string(),
        );

        // Simplify mode and status strings
        let mode = details.mode.as_deref().map(|mode_str| {
            if mode_str.contains("inperson") {
                "inperson".to_string()
            } else if mode_str.contains("virtual") {
                "virtual".to_string()
            } else if mode_str.contains("hybrid") {
                "hybrid".to_string()
            } else {
                mode_str.to_string()
            }
        });

        let status = details.status.as_deref().map(|status_str| {
            if status_str.contains("planned") {
                "planned".to_string()
            } else if status_str.contains("scheduled") {
                "scheduled".to_string()
            } else if status_str.contains("rescheduled") {
                "rescheduled".to_string()
            } else if status_str.contains("cancelled") {
                "cancelled".to_string()
            } else if status_str.contains("postponed") {
                "postponed".to_string()
            } else {
                status_str.to_string()
            }
        });

        let name = Some(event_name);
        let description = event_description;
        let starts_at = details.starts_at;
        let ends_at = details.ends_at;

        let name = name.ok_or(EventViewError::MissingEventName)?;

        let description_short = description
            .as_ref()
            .map(|value| truncate_text(value, 200, Some("...".to_string())).to_string());

        let starts_at_human = starts_at.as_ref().map(|value| {
            value
                .with_timezone(&tz)
                .format("%e %B %Y %I:%M %P %Z")
                .to_string()
        });
        let starts_at_machine = starts_at
            .as_ref()
            .map(|value| value.with_timezone(&tz).to_string());

        let ends_at_machine = ends_at.as_ref().map(|value| {
            value
                .with_timezone(&tz)
                .format("%e %B %Y %I:%M %P %Z")
                .to_string()
        });
        let ends_at_human = ends_at
            .as_ref()
            .map(|value| value.with_timezone(&tz).to_string());

        let site_url = if event.lexicon == LexiconCommunityEventNSID {
            format!("/{}/{}", repository, rkey)
        } else {
            format!("/{}/{}?collection={}", repository, rkey, event.lexicon)
        };

        // Format address if an Address location is found
        let address_display = details.locations.iter()
            .filter_map(|loc| {
                if let crate::atproto::lexicon::community::lexicon::calendar::event::EventLocation::Address(address) = loc {
                    Some(crate::storage::event::format_address(address))
                } else {
                    None
                }
            })
            .next(); // Take the first address found

        // Extract links from EventLink objects
        let links = details.uris.iter()
            .map(|uri| {
                match uri {
                    crate::atproto::lexicon::community::lexicon::calendar::event::EventLink::Current { uri, name } => {
                        (uri.clone(), name.clone())
                    }
                }
            })
            .collect::<Vec<_>>();

        Ok(EventView {
            site_url,
            aturi: event.aturi.clone(),
            cid: event.cid.clone(),
            repository,
            collection,
            organizer_did,
            organizer_display_name,
            starts_at_machine,
            starts_at_human,
            ends_at_machine,
            ends_at_human,
            name,
            description,
            description_short,
            count_going: 0,
            count_notgoing: 0,
            count_interested: 0,
            mode,
            status,
            address_display,
            links,
        })
    }
}

pub async fn hydrate_event_organizers(
    pool: &StoragePool,
    events: &[EventWithRole],
) -> Result<HashMap<std::string::String, Handle>> {
    if events.is_empty() {
        return Ok(HashMap::default());
    }
    let event_creator_dids = events
        .iter()
        .map(|event| event.event.did.clone())
        .collect::<Vec<_>>();
    handles_by_did(pool, event_creator_dids)
        .await
        .map_err(|err| err.into())
}

pub async fn hydrate_event_rsvp_counts(
    pool: &StoragePool,
    events: &mut [EventView],
) -> Result<(), anyhow::Error> {
    if events.is_empty() {
        return Ok(());
    }
    let aturis = events.iter().map(|e| e.aturi.clone()).collect::<Vec<_>>();
    let res = get_event_rsvp_counts(pool, aturis).await;

    match res {
        Ok(counts) => {
            for event in events.iter_mut() {
                let key_going = (event.aturi.clone(), "going".to_string());
                let key_interested = (event.aturi.clone(), "interested".to_string());
                let key_notgoing = (event.aturi.clone(), "notgoing".to_string());

                event.count_going = counts.get(&key_going).cloned().unwrap_or(0) as u32;
                event.count_interested = counts.get(&key_interested).cloned().unwrap_or(0) as u32;
                event.count_notgoing = counts.get(&key_notgoing).cloned().unwrap_or(0) as u32;
            }
            Ok(())
        }
        Err(StorageError::CannotBeginDatabaseTransaction(_)) => {
            // Fall back to individual counts if the batched query fails
            for event in events.iter_mut() {
                event.count_going = count_event_rsvps(pool, &event.aturi, "going")
                    .await
                    .unwrap_or_default();
                event.count_interested = count_event_rsvps(pool, &event.aturi, "interested")
                    .await
                    .unwrap_or_default();
                event.count_notgoing = count_event_rsvps(pool, &event.aturi, "notgoing")
                    .await
                    .unwrap_or_default();
            }
            Ok(())
        }
        Err(e) => Err(EventViewError::FailedToHydrateRsvpCounts(e.to_string()).into()),
    }
}
