use anyhow::Result;
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::Cached;
use chrono::Utc;
use http::StatusCode;
use metrohash::MetroHash64;
use minijinja::context as template_context;
use std::hash::Hasher;

use crate::{
    atproto::{
        auth::SimpleOAuthSessionProvider,
        client::{OAuthPdsClient, PutRecordRequest},
        lexicon::{
            com::atproto::repo::StrongRef,
            community::lexicon::calendar::rsvp::{Rsvp, RsvpStatus, NSID as RSVP_COLLECTION},
            events::smokesignal::calendar::event::NSID as EVENT_COLLECTION,
        },
    },
    contextual_error,
    http::{
        context::WebContext,
        errors::{MigrateRsvpError, WebError},
        middleware_auth::Auth,
        middleware_i18n::Language,
    },
    resolve::{parse_input, InputType},
    select_template,
    storage::{
        event::{event_get, get_user_rsvp, rsvp_insert},
        handle::{handle_for_did, handle_for_handle, model::Handle},
    },
};

/// Migrates a user's RSVP from a legacy event to a standard event format.
///
/// This handler takes an existing RSVP from the legacy SmokeSignal event format
/// and creates a new RSVP record that follows the standard Community Calendar lexicon.
/// It only migrates if the user has an RSVP for the legacy event and does not yet have
/// an RSVP for the standard event.
///
/// The process involves:
/// 1. Verifying both legacy and standard events exist
/// 2. Checking if the user has RSVP'd to the legacy event
/// 3. Ensuring the user doesn't already have an RSVP for the standard event
/// 4. Creating a new RSVP record in the standard format
/// 5. Storing the RSVP both on the PDS and in the local database
pub async fn handle_migrate_rsvp(
    State(web_context): State<WebContext>,
    Language(language): Language,
    Cached(auth): Cached<Auth>,
    Path((handle_slug, event_rkey)): Path<(String, String)>,
) -> Result<impl IntoResponse, WebError> {
    // Require user to be logged in
    let current_handle = auth.require(
        &web_context.config.destination_key,
        "/{handle_slug}/{event_rkey}/migrate-rsvp",
    )?;

    let default_context = template_context! {
        language => language.to_string(),
        current_handle => current_handle.clone(),
        canonical_url => format!("https://{}/{}/{}", web_context.config.external_base, handle_slug, event_rkey),
    };

    let error_template = select_template!(false, false, language);

    // Get handle information from the path parameter
    let profile: Result<Handle> = match parse_input(&handle_slug) {
        Ok(InputType::Handle(handle)) => handle_for_handle(&web_context.pool, &handle)
            .await
            .map_err(|err| err.into()),
        Ok(InputType::Plc(did) | InputType::Web(did)) => handle_for_did(&web_context.pool, &did)
            .await
            .map_err(|err| err.into()),
        Err(err) => Err(err.into()),
    };

    let profile = match profile {
        Ok(profile) => profile,
        Err(err) => {
            return contextual_error!(
                web_context,
                language,
                error_template,
                default_context,
                err,
                StatusCode::NOT_FOUND
            );
        }
    };

    // Construct AT-URIs for both versions of the event
    // Legacy event uses the SmokeSignal specific event collection (events.smokesignal.calendar.event)
    let legacy_event_aturi = format!("at://{}/{}/{}", profile.did, EVENT_COLLECTION, event_rkey);

    // Standard event uses the community lexicon event collection (community.lexicon.calendar.event)
    // We need to replace "events.smokesignal" with "community.lexicon" but keep "calendar.event"
    let standard_event_collection = "community.lexicon.calendar.event";
    let standard_event_aturi = format!(
        "at://{}/{}/{}",
        profile.did, standard_event_collection, event_rkey
    );

    // Verify that the legacy event exists
    if let Err(err) = event_get(&web_context.pool, &legacy_event_aturi).await {
        return contextual_error!(
            web_context,
            language,
            error_template,
            default_context,
            err,
            StatusCode::NOT_FOUND
        );
    }

    // Retrieve the standard event to ensure it exists and to get its CID
    // This is crucial because we'll reference this event in the RSVP
    let standard_event = match event_get(&web_context.pool, &standard_event_aturi).await {
        Ok(event) => event,
        Err(err) => {
            return contextual_error!(
                web_context,
                language,
                error_template,
                default_context,
                err,
                StatusCode::NOT_FOUND
            );
        }
    };

    // Verify that current user is the event creator or has RSVP'd
    // (no need to restrict to event creator for RSVP migration)

    // Check if user has RSVP'd to the legacy event - this is required to proceed
    // We need to get the RSVP status to create the same status in the standard format
    let user_legacy_rsvp_status =
        match get_user_rsvp(&web_context.pool, &legacy_event_aturi, &current_handle.did).await {
            Ok(Some(status)) => status,
            Ok(None) => {
                // No legacy RSVP found, redirect back to the event view page
                // Migration is only necessary if there's an RSVP to migrate
                return Ok(
                    Redirect::to(&format!("/{}/{}", handle_slug, event_rkey)).into_response()
                );
            }
            Err(err) => {
                return contextual_error!(
                    web_context,
                    language,
                    error_template,
                    default_context,
                    err
                );
            }
        };

    // Check if user already has an RSVP for the standard event using the community lexicon format
    // Skip migration if they already have a standard RSVP to avoid duplicates
    let has_standard_rsvp = match get_user_rsvp(
        &web_context.pool,
        &standard_event_aturi,
        &current_handle.did,
    )
    .await
    {
        Ok(Some(_)) => true,
        Ok(None) => false,
        Err(err) => {
            return contextual_error!(web_context, language, error_template, default_context, err);
        }
    };

    // If user already has an RSVP for the standard event, just redirect
    if has_standard_rsvp {
        return Ok(Redirect::to(&format!("/{}/{}", handle_slug, event_rkey)).into_response());
    }

    // Create a new RSVP for the standard event
    // Error if we don't have auth data
    let auth_data = auth.1.ok_or(MigrateRsvpError::NotAuthorized)?;
    let client_auth: SimpleOAuthSessionProvider = SimpleOAuthSessionProvider::try_from(auth_data)?;

    let client = OAuthPdsClient {
        http_client: &web_context.http_client,
        pds: &current_handle.pds,
    };

    // Create a reference to the standard event that will be the subject of the RSVP
    let subject = StrongRef {
        uri: standard_event_aturi.clone(),
        cid: standard_event.cid.clone(),
    };

    // Convert RSVP status string to enum
    // Map from legacy SmokeSignal RSVP status to Community Calendar lexicon RSVP status
    // The status names match but they use different namespaces in the actual enum variants
    let status = match user_legacy_rsvp_status.as_str() {
        "going" => RsvpStatus::Going, // "events.smokesignal.calendar.rsvp#going" -> "community.lexicon.calendar.rsvp#going"
        "interested" => RsvpStatus::Interested, // "events.smokesignal.calendar.rsvp#interested" -> "community.lexicon.calendar.rsvp#interested"
        "notgoing" => RsvpStatus::NotGoing, // "events.smokesignal.calendar.rsvp#notgoing" -> "community.lexicon.calendar.rsvp#notgoing"
        _ => {
            return contextual_error!(
                web_context,
                language,
                error_template,
                default_context,
                MigrateRsvpError::InvalidRsvpStatus(user_legacy_rsvp_status.clone())
            );
        }
    };

    // Generate a deterministic record key based on the event URI
    // This ensures the same RSVP can't be created multiple times
    // The key is a hash of the event URI to prevent duplicate RSVPs for the same event
    let mut h = MetroHash64::default();
    h.write(subject.uri.clone().as_bytes());
    let record_key = crockford::encode(h.finish());

    // Create new RSVP record with the same status as the legacy RSVP
    // but using the standard Community Calendar format
    // Note: The standard Rsvp::Current format requires a non-optional created_at timestamp
    let now = Utc::now();
    let rsvp_record_content = Rsvp::Current {
        created_at: now, // Set to current time as we're creating a new record
        subject,
        status,
    };

    // Send the RSVP to the PDS (Personal Data Server)
    let rsvp_record = PutRecordRequest {
        repo: current_handle.did.clone(),
        collection: RSVP_COLLECTION.to_string(),
        validate: false,
        record_key,
        record: rsvp_record_content.clone(),
        swap_commit: None,
        swap_record: None,
    };

    let put_record_result = client.put_record(&client_auth, rsvp_record).await;

    if let Err(err) = put_record_result {
        return contextual_error!(web_context, language, error_template, default_context, err);
    }

    // put_record_result is guaranteed to be Ok here since we checked for Err above
    let create_record_result = put_record_result?;

    // Store the new RSVP in the database
    let rsvp_insert_result = rsvp_insert(
        &web_context.pool,
        &create_record_result.uri,
        &create_record_result.cid,
        &current_handle.did,
        RSVP_COLLECTION,
        &rsvp_record_content,
    )
    .await;

    if let Err(err) = rsvp_insert_result {
        return contextual_error!(web_context, language, error_template, default_context, err);
    }

    // Redirect to the event view page
    Ok(Redirect::to(&format!("/{}/{}", handle_slug, event_rkey)).into_response())
}
