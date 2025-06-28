use std::fmt;

use anyhow::Result;
use axum::{
    extract::{Path, Query},
    response::{IntoResponse, Redirect},
};
use axum_htmx::HxBoosted;
use axum_template::RenderHtml;
use http::StatusCode;
use minijinja::context as template_context;
use serde::{Deserialize, Serialize};

use crate::atproto::lexicon::community::lexicon::calendar::event::NSID;
use crate::atproto::lexicon::events::smokesignal::calendar::event::NSID as SMOKESIGNAL_EVENT_NSID;
use crate::contextual_error;
use crate::http::context::UserRequestContext;
use crate::http::errors::CommonError;
use crate::http::errors::ViewEventError;
use crate::http::errors::WebError;
use crate::http::event_view::hydrate_event_rsvp_counts;
use crate::http::event_view::EventView;
use crate::http::pagination::Pagination;
use crate::http::tab_selector::TabSelector;
use crate::http::utils::url_from_aturi;
use crate::resolve::parse_input;
use crate::resolve::InputType;
use crate::select_template;
use crate::storage::event::count_event_rsvps;
use crate::storage::event::event_exists;
use crate::storage::event::event_get;
use crate::storage::event::get_event_rsvps;
use crate::storage::event::get_user_rsvp;
use crate::storage::handle::handle_for_did;
use crate::storage::handle::handle_for_handle;
use crate::storage::handle::model::Handle;
use crate::storage::StoragePool;

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub enum RSVPTab {
    Going,
    Interested,
    NotGoing,
}

impl fmt::Display for RSVPTab {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RSVPTab::Going => write!(f, "going"),
            RSVPTab::Interested => write!(f, "interested"),
            RSVPTab::NotGoing => write!(f, "notgoing"),
        }
    }
}

impl From<TabSelector> for RSVPTab {
    fn from(tab_selector: TabSelector) -> Self {
        match tab_selector.tab.clone().unwrap_or_default().as_str() {
            "interested" => RSVPTab::Interested,
            "notgoing" => RSVPTab::NotGoing,
            _ => RSVPTab::Going,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CollectionParam {
    #[serde(default = "default_collection")]
    collection: String,
}

fn default_collection() -> String {
    NSID.to_string()
}

/// Helper function to fetch the organizer's handle (which contains their time zone)
/// This is used to implement the time zone selection logic.
async fn fetch_organizer_handle(pool: &StoragePool, did: &str) -> Option<Handle> {
    match handle_for_did(pool, did).await {
        Ok(handle) => Some(handle),
        Err(err) => {
            tracing::warn!("Failed to fetch organizer handle: {}", err);
            None
        }
    }
}

pub async fn handle_view_event(
    ctx: UserRequestContext,
    HxBoosted(hx_boosted): HxBoosted,
    Path((handle_slug, event_rkey)): Path<(String, String)>,
    pagination: Query<Pagination>,
    tab_selector: Query<TabSelector>,
    collection_param: Query<CollectionParam>,
) -> Result<impl IntoResponse, WebError> {
    let default_context = template_context! {
        language => ctx.language.to_string(),
        current_handle => ctx.current_handle,
    };

    let render_template = select_template!("view_event", hx_boosted, false, ctx.language);
    let error_template = select_template!(hx_boosted, false, ctx.language);

    let profile: Result<Handle, WebError> = match parse_input(&handle_slug) {
        Ok(InputType::Handle(handle)) => handle_for_handle(&ctx.web_context.pool, &handle)
            .await
            .map_err(|err| err.into()),
        Ok(InputType::Plc(did) | InputType::Web(did)) => {
            handle_for_did(&ctx.web_context.pool, &did)
                .await
                .map_err(|err| err.into())
        }
        _ => Err(CommonError::InvalidHandleSlug.into()),
    };

    if let Err(err) = profile {
        return contextual_error!(
            ctx.web_context,
            ctx.language,
            error_template,
            default_context,
            err,
            StatusCode::NOT_FOUND
        );
    }

    let profile = profile.unwrap();

    // We'll use TimeZoneSelector to implement the time zone selection logic
    // The timezone selection will happen after we fetch the event

    // Use the provided collection parameter instead of the default NSID
    let collection = &collection_param.0.collection;
    let lookup_aturi = format!("at://{}/{}/{}", profile.did, collection, event_rkey);

    // Check if this is a legacy event (not using the standard community calendar collection)
    let is_legacy_event = collection != NSID;

    // If this is a legacy event, check if a standard version exists
    // If this is a standard event, check if a legacy version exists (migrated event)
    let standard_event_exists;
    let has_been_migrated;

    if is_legacy_event {
        // This is a legacy event, check if a standard version exists
        let standard_aturi = format!("at://{}/{}/{}", profile.did, NSID, event_rkey);

        // Try to fetch the standard event
        standard_event_exists = match event_get(&ctx.web_context.pool, &standard_aturi).await {
            Ok(_) => {
                tracing::info!("Standard version of legacy event found: {}", standard_aturi);
                true
            }
            Err(_) => {
                tracing::info!("No standard version found for legacy event");
                false
            }
        };
        // Legacy events are never migrated
        has_been_migrated = false;
    } else {
        // This is a standard event, so there's no standard version to check for
        standard_event_exists = false;

        // Check if this is a migrated event (i.e., a legacy version exists)
        let legacy_aturi = format!(
            "at://{}/{}/{}",
            profile.did, SMOKESIGNAL_EVENT_NSID, event_rkey
        );
        has_been_migrated = match event_get(&ctx.web_context.pool, &legacy_aturi).await {
            Ok(_) => {
                tracing::info!(
                    "Legacy version found for standard event - this is a migrated event: {}",
                    legacy_aturi
                );
                true
            }
            Err(_) => {
                tracing::info!("No legacy version found for standard event");
                false
            }
        };
    };

    // Try to get the event from the requested collection
    let event_get_result = event_get(&ctx.web_context.pool, &lookup_aturi).await;

    let event_result = match &event_get_result {
        Ok(event) => {
            let organizer_handle = {
                if ctx
                    .current_handle
                    .clone()
                    .is_some_and(|h| h.did == event.did)
                {
                    ctx.current_handle.clone()
                } else {
                    fetch_organizer_handle(&ctx.web_context.pool, &event.did).await
                }
            };

            EventView::try_from((
                ctx.current_handle.as_ref(),
                organizer_handle.as_ref(),
                event,
            ))
        }
        Err(err) => Err(ViewEventError::EventNotFound(err.to_string()).into()),
    };

    // If event not found and using default collection, try fallback collection
    if event_result.is_err() && collection == NSID {
        // Check if event exists in fallback collection
        let fallback_aturi = format!(
            "at://{}/{}/{}",
            profile.did, SMOKESIGNAL_EVENT_NSID, event_rkey
        );
        tracing::info!(
            "Event not found in default collection, trying fallback: {}",
            fallback_aturi
        );

        // Try to fetch from fallback collection
        let fallback_result: Result<bool, WebError> =
            event_exists(&ctx.web_context.pool, &fallback_aturi)
                .await
                .map_err(|err| ViewEventError::FallbackFailed(err.to_string()).into());

        match fallback_result {
            Ok(true) => {
                // HTTP 307 temporary redirect
                let encoded_collection = urlencoding::encode(SMOKESIGNAL_EVENT_NSID).to_string();
                let uri = format!(
                    "/{}/{}?collection={}",
                    handle_slug, event_rkey, encoded_collection
                );
                return Ok(Redirect::to(&uri).into_response());
            }
            Err(err) => {
                tracing::error!(fallback_aturi, err = ?err, "failed to lookup fallback_aturi: {}", err);
            }
            _ => {}
        }
    }

    if let Err(err) = event_result {
        return contextual_error!(
            ctx.web_context,
            ctx.language,
            error_template,
            default_context,
            err,
            StatusCode::NOT_FOUND
        );
    }

    let mut event = event_result.unwrap();

    // Hydrate event organizer display name
    let mut event_vec = vec![event];

    // if let Err(err) = hydrate_events(&ctx.web_context.pool, &mut event_vec).await {
    //     tracing::warn!("Failed to hydrate event organizers: {}", err);
    // }

    if let Err(err) = hydrate_event_rsvp_counts(&ctx.web_context.pool, &mut event_vec).await {
        tracing::warn!("Failed to hydrate event counts: {}", err);
    }

    event = event_vec.remove(0);

    let is_self = ctx
        .current_handle
        .clone()
        .is_some_and(|inner_current_entity| inner_current_entity.did == profile.did);

    let (_page, _page_size) = pagination.clamped();
    let tab: RSVPTab = tab_selector.0.into();
    let tab_name = tab.to_string();

    let event_url = url_from_aturi(&ctx.web_context.config.external_base, &event.aturi)?;

    // Add Edit button link if the user is the event creator
    let can_edit = ctx
        .current_handle
        .clone()
        .is_some_and(|current_entity| current_entity.did == profile.did);

    // Variables for RSVP data
    let (
        user_rsvp_status,
        going_count,
        interested_count,
        notgoing_count,
        going_handles,
        interested_handles,
        notgoing_handles,
        user_has_standard_rsvp,
    ) = if !is_legacy_event {
        // Only fetch RSVP data for standard (non-legacy) events
        // Get user's RSVP status if logged in
        let user_rsvp = if let Some(current_entity) = &ctx.current_handle {
            match get_user_rsvp(&ctx.web_context.pool, &lookup_aturi, &current_entity.did).await {
                Ok(status) => status,
                Err(err) => {
                    tracing::error!("Error getting user RSVP status: {:?}", err);
                    None
                }
            }
        } else {
            None
        };

        // Get counts for all RSVP statuses
        let going_count = count_event_rsvps(&ctx.web_context.pool, &lookup_aturi, "going")
            .await
            .unwrap_or_default();

        let interested_count =
            count_event_rsvps(&ctx.web_context.pool, &lookup_aturi, "interested")
                .await
                .unwrap_or_default();

        let notgoing_count = count_event_rsvps(&ctx.web_context.pool, &lookup_aturi, "notgoing")
            .await
            .unwrap_or_default();

        // Only get handles for the active tab
        let (going_handles, interested_handles, notgoing_handles) = match tab {
            RSVPTab::Going => {
                let rsvps = get_event_rsvps(&ctx.web_context.pool, &lookup_aturi, Some("going"))
                    .await
                    .unwrap_or_default();

                let mut handles = Vec::new();
                for (did, _) in &rsvps {
                    if let Ok(handle) = handle_for_did(&ctx.web_context.pool, did).await {
                        handles.push(handle.handle);
                    }
                }
                (handles, Vec::new(), Vec::new())
            }
            RSVPTab::Interested => {
                let rsvps =
                    get_event_rsvps(&ctx.web_context.pool, &lookup_aturi, Some("interested"))
                        .await
                        .unwrap_or_default();

                let mut handles = Vec::new();
                for (did, _) in &rsvps {
                    if let Ok(handle) = handle_for_did(&ctx.web_context.pool, did).await {
                        handles.push(handle.handle);
                    }
                }
                (Vec::new(), handles, Vec::new())
            }
            RSVPTab::NotGoing => {
                let rsvps = get_event_rsvps(&ctx.web_context.pool, &lookup_aturi, Some("notgoing"))
                    .await
                    .unwrap_or_default();

                let mut handles = Vec::new();
                for (did, _) in &rsvps {
                    if let Ok(handle) = handle_for_did(&ctx.web_context.pool, did).await {
                        handles.push(handle.handle);
                    }
                }
                (Vec::new(), Vec::new(), handles)
            }
        };

        (
            user_rsvp,
            going_count,
            interested_count,
            notgoing_count,
            going_handles,
            interested_handles,
            notgoing_handles,
            false, // Not used for standard events
        )
    } else {
        // For legacy events, still check if the user has RSVP'd
        let user_rsvp = if let Some(current_entity) = &ctx.current_handle {
            match get_user_rsvp(&ctx.web_context.pool, &lookup_aturi, &current_entity.did).await {
                Ok(status) => status,
                Err(err) => {
                    tracing::error!("Error getting user RSVP status for legacy event: {:?}", err);
                    None
                }
            }
        } else {
            None
        };

        // If this is a legacy event, check if the user already has an RSVP for the standard version
        // to avoid showing the migrate button unnecessarily
        let user_has_standard_rsvp =
            if standard_event_exists && user_rsvp.is_some() && ctx.current_handle.is_some() {
                // Construct the standard event URI
                let standard_event_uri = format!("at://{}/{}/{}", profile.did, NSID, event_rkey);

                // Check if the user has an RSVP for the standard event
                match get_user_rsvp(
                    &ctx.web_context.pool,
                    &standard_event_uri,
                    &ctx.current_handle.as_ref().unwrap().did,
                )
                .await
                {
                    Ok(Some(_)) => {
                        tracing::info!(
                            "User already has an RSVP for the standard event: {}",
                            standard_event_uri
                        );
                        true
                    }
                    Ok(None) => false,
                    Err(err) => {
                        tracing::error!(
                            "Error checking if user has RSVP for standard event: {:?}",
                            err
                        );
                        false // Default to false to allow migration attempt if we can't determine
                    }
                }
            } else {
                false
            };

        tracing::info!("Legacy event detected, only fetching user RSVP status");
        (
            user_rsvp,
            0,
            0,
            0,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            user_has_standard_rsvp,
        )
    };

    // Set counts on event
    let mut event_with_counts = event;
    event_with_counts.count_going = going_count;
    event_with_counts.count_interested = interested_count;
    event_with_counts.count_notgoing = notgoing_count;

    Ok((
        StatusCode::OK,
        RenderHtml(
            &render_template,
            ctx.web_context.engine.clone(),
            template_context! {
                current_handle => ctx.current_handle,
                language => ctx.language.to_string(),
                canonical_url => event_url,
                event => event_with_counts,
                is_self,
                can_edit,
                going => going_handles,
                interested => interested_handles,
                notgoing => notgoing_handles,
                active_tab => tab_name,
                user_rsvp_status,
                handle_slug,
                event_rkey,
                collection => collection.clone(),
                is_legacy_event,
                standard_event_exists,
                has_been_migrated,
                user_has_standard_rsvp,
                standard_event_url => if standard_event_exists {
                    Some(format!("/{}/{}", handle_slug, event_rkey))
                } else {
                    None
                },
                SMOKESIGNAL_EVENT_NSID => SMOKESIGNAL_EVENT_NSID,
                using_SMOKESIGNAL_EVENT_NSID => collection == SMOKESIGNAL_EVENT_NSID,
            },
        ),
    )
        .into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    // No imports needed for basic unit tests

    // Simple unit test for the RSVPTab conversion
    #[test]
    fn test_rsrvp_tab_from_tab_selector() {
        let tab_selector = TabSelector {
            tab: Some("going".to_string()),
        };
        let rsvp_tab = RSVPTab::from(tab_selector);
        assert_eq!(rsvp_tab, RSVPTab::Going);

        let tab_selector = TabSelector {
            tab: Some("interested".to_string()),
        };
        let rsvp_tab = RSVPTab::from(tab_selector);
        assert_eq!(rsvp_tab, RSVPTab::Interested);

        let tab_selector = TabSelector {
            tab: Some("notgoing".to_string()),
        };
        let rsvp_tab = RSVPTab::from(tab_selector);
        assert_eq!(rsvp_tab, RSVPTab::NotGoing);

        // Default case
        let tab_selector = TabSelector { tab: None };
        let rsvp_tab = RSVPTab::from(tab_selector);
        assert_eq!(rsvp_tab, RSVPTab::Going);
    }

    #[test]
    fn test_rsvp_tab_display() {
        assert_eq!(RSVPTab::Going.to_string(), "going");
        assert_eq!(RSVPTab::Interested.to_string(), "interested");
        assert_eq!(RSVPTab::NotGoing.to_string(), "notgoing");
    }

    // Test collection parameter default
    #[test]
    fn test_collection_param_default() {
        assert_eq!(default_collection(), NSID);
    }
}
