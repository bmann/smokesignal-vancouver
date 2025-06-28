use anyhow::Result;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
};
use axum_extra::extract::Cached;
use axum_htmx::{HxBoosted, HxRequest};
use axum_template::RenderHtml;
use http::StatusCode;
use minijinja::context as template_context;
use std::collections::HashMap;

use crate::{
    atproto::{
        auth::SimpleOAuthSessionProvider,
        client::{OAuthPdsClient, PutRecordRequest},
        lexicon::{
            community::lexicon::calendar::event::{
                Event as CommunityEvent, EventLink, EventLocation as CommunityLocation, Mode,
                Status, NSID as COMMUNITY_NSID,
            },
            community::lexicon::location,
            events::smokesignal::calendar::event::{
                Event as SmokeSignalEvent, Location as SmokeSignalLocation, PlaceLocation,
                NSID as SMOKESIGNAL_NSID,
            },
        },
    },
    contextual_error,
    http::{
        context::WebContext, errors::MigrateEventError, errors::WebError, middleware_auth::Auth,
        middleware_i18n::Language, utils::url_from_aturi,
    },
    resolve::{parse_input, InputType},
    select_template,
    storage::{
        event::{event_get, event_insert_with_metadata},
        handle::{handle_for_did, handle_for_handle, model::Handle},
    },
};

pub async fn handle_migrate_event(
    State(web_context): State<WebContext>,
    HxBoosted(hx_boosted): HxBoosted,
    Language(language): Language,
    Cached(auth): Cached<Auth>,
    HxRequest(hx_request): HxRequest,
    Path((handle_slug, event_rkey)): Path<(String, String)>,
) -> Result<impl IntoResponse, WebError> {
    let current_handle = auth.require(&web_context.config.destination_key, "/")?;

    // Configure templates
    let default_context = template_context! {
        current_handle,
        language => language.to_string(),
        canonical_url => format!("https://{}/{}/{}/migrate", web_context.config.external_base, handle_slug, event_rkey),
    };
    let render_template = select_template!("migrate_event", hx_boosted, hx_request, language);
    let error_template = select_template!(hx_boosted, hx_request, language);

    // Lookup the user handle/profile
    let profile: Result<Handle> = match parse_input(&handle_slug) {
        Ok(InputType::Handle(handle)) => handle_for_handle(&web_context.pool, &handle)
            .await
            .map_err(|err| err.into()),
        Ok(InputType::Plc(did) | InputType::Web(did)) => handle_for_did(&web_context.pool, &did)
            .await
            .map_err(|err| err.into()),
        Err(err) => Err(err.into()),
    };

    if let Err(err) = profile {
        return contextual_error!(
            web_context,
            language,
            error_template,
            default_context,
            err,
            StatusCode::NOT_FOUND
        );
    }
    let profile = profile.unwrap();

    // Construct AT URI for the source event
    let source_aturi = format!("at://{}/{}/{}", profile.did, SMOKESIGNAL_NSID, event_rkey);

    // Check if the user is authorized to migrate this event (must be the event creator/organizer)
    if profile.did != current_handle.did {
        return contextual_error!(
            web_context,
            language,
            error_template,
            default_context,
            MigrateEventError::NotAuthorized,
            StatusCode::FORBIDDEN
        );
    }

    // Fetch the event from the database
    let event = event_get(&web_context.pool, &source_aturi).await;
    if let Err(err) = event {
        return contextual_error!(
            web_context,
            language,
            error_template,
            default_context,
            err,
            StatusCode::OK
        );
    }
    let event = event.unwrap();

    // Check that this is a smokesignal event (we only migrate those)
    if event.lexicon != SMOKESIGNAL_NSID {
        // If it's already the community event type, we don't need to migrate
        if event.lexicon == COMMUNITY_NSID {
            return contextual_error!(
                web_context,
                language,
                error_template,
                default_context,
                MigrateEventError::AlreadyMigrated,
                StatusCode::BAD_REQUEST
            );
        }

        return contextual_error!(
            web_context,
            language,
            error_template,
            default_context,
            MigrateEventError::UnsupportedEventType,
            StatusCode::BAD_REQUEST
        );
    }

    // Parse the legacy event
    let legacy_event = match serde_json::from_value::<SmokeSignalEvent>(event.record.0.clone()) {
        Ok(event) => event,
        Err(err) => {
            return contextual_error!(
                web_context,
                language,
                error_template,
                default_context,
                err,
                StatusCode::BAD_REQUEST
            );
        }
    };

    // Extract data from the legacy event
    let (name, text, created_at, starts_at, extra) = match legacy_event {
        SmokeSignalEvent::Current {
            name,
            text,
            created_at,
            starts_at,
            extra,
        } => (name, text, created_at, starts_at, extra),
    };

    // Extract optional fields from the extra map
    let ends_at = extra
        .get("endsAt")
        .and_then(|v| v.as_str())
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc));

    // Extract legacy mode/status
    let legacy_mode = extra.get("mode").and_then(|v| v.as_str());
    let legacy_status = extra.get("status").and_then(|v| v.as_str());

    // Convert mode to the community format
    let mode = match legacy_mode {
        Some("events.smokesignal.calendar.event#inperson") => Some(Mode::InPerson),
        Some("events.smokesignal.calendar.event#virtual") => Some(Mode::Virtual),
        _ => None,
    };

    // Convert status to the community format
    let status = match legacy_status {
        Some("events.smokesignal.calendar.event#scheduled") => Some(Status::Scheduled),
        Some("events.smokesignal.calendar.event#cancelled") => Some(Status::Cancelled),
        Some("events.smokesignal.calendar.event#postponed") => Some(Status::Postponed),
        Some("events.smokesignal.calendar.event#rescheduled") => Some(Status::Rescheduled),
        _ => Some(Status::Scheduled), // Default to scheduled if not specified
    };

    // Helper function to convert PlaceLocation to community Address
    fn convert_place_to_address(place: &PlaceLocation) -> location::Address {
        location::Address::Current {
            country: place.country.clone().unwrap_or_default(),
            postal_code: place.postal_code.clone(),
            region: place.region.clone(),
            locality: place.locality.clone(),
            street: place.street.clone(),
            name: Some(place.name.clone()),
        }
    }

    // Extract locations and links from the legacy event
    let mut locations = Vec::new();
    let mut uris = Vec::new();

    if let Some(location_values) = extra.get("location") {
        if let Some(location_array) = location_values.as_array() {
            for location_value in location_array {
                // Parse the location
                if let Ok(location) =
                    serde_json::from_value::<SmokeSignalLocation>(location_value.clone())
                {
                    match location {
                        SmokeSignalLocation::Place(place) => {
                            // Convert place location to community address
                            let address = convert_place_to_address(&place);
                            locations.push(CommunityLocation::Address(address));
                        }
                        SmokeSignalLocation::Virtual(virtual_loc) => {
                            // Convert virtual locations to EventLink elements
                            if let Some(url) = &virtual_loc.url {
                                uris.push(EventLink::Current {
                                    uri: url.clone(),
                                    name: Some(virtual_loc.name.clone()),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // Create a new community event
    let new_event = CommunityEvent::Current {
        name: name.clone(),
        description: text.unwrap_or_default(),
        created_at: created_at.unwrap_or_else(chrono::Utc::now),
        starts_at,
        ends_at,
        mode,
        status,
        locations,
        uris,
        extra: HashMap::default(),
    };

    // Construct the target AT-URI for the new community event
    let migrated_aturi = format!("at://{}/{}/{}", profile.did, COMMUNITY_NSID, event_rkey);

    // Check if a record already exists at the target AT-URI
    let existing_event = event_get(&web_context.pool, &migrated_aturi).await;
    if existing_event.is_ok() {
        return contextual_error!(
            web_context,
            language,
            error_template,
            default_context,
            MigrateEventError::DestinationExists,
            StatusCode::CONFLICT
        );
    }

    // Set up XRPC client
    // Error if we don't have auth data
    let auth_data = auth.1.ok_or(MigrateEventError::NotAuthorized)?;
    let client_auth: SimpleOAuthSessionProvider = SimpleOAuthSessionProvider::try_from(auth_data)?;

    let client = OAuthPdsClient {
        http_client: &web_context.http_client,
        pds: &current_handle.pds,
    };

    // Create the community event record in the user's PDS using putRecord to retain the same rkey
    let update_record_request = PutRecordRequest {
        repo: current_handle.did.clone(),
        collection: COMMUNITY_NSID.to_string(),
        record_key: event_rkey.clone(),
        record: new_event.clone(),
        validate: false,
        swap_commit: None,
        swap_record: None, // We're creating a new record, not replacing
    };

    // Write to the PDS
    let update_record_result = client.put_record(&client_auth, update_record_request).await;
    if let Err(err) = update_record_result {
        return contextual_error!(
            web_context,
            language,
            error_template,
            default_context,
            err,
            StatusCode::OK
        );
    }
    // update_record_result is guaranteed to be Ok at this point since we checked for Err above
    let update_record_result = update_record_result?;

    // We already have the migrated AT-URI defined above

    // Insert the migrated event into the database
    let migrated_event_insert_result = event_insert_with_metadata(
        &web_context.pool,
        &migrated_aturi,
        &update_record_result.cid,
        &current_handle.did,
        COMMUNITY_NSID,
        &new_event,
        &name,
    )
    .await;

    if let Err(err) = migrated_event_insert_result {
        return contextual_error!(
            web_context,
            language,
            error_template,
            default_context,
            err,
            StatusCode::OK
        );
    }

    // Generate URL for the migrated event
    let migrated_event_url = url_from_aturi(&web_context.config.external_base, &migrated_aturi)?;

    // Return success with migration complete template
    Ok(RenderHtml(
        &render_template,
        web_context.engine.clone(),
        template_context! {
            migrated_event_url => migrated_event_url,
            source_aturi => source_aturi,
            migrated_aturi => migrated_aturi,
            event_name => name,
            source_lexicon => SMOKESIGNAL_NSID,
            target_lexicon => COMMUNITY_NSID,
            ..default_context
        },
    )
    .into_response())
}
