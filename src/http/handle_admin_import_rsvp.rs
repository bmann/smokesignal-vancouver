use anyhow::Result;
use axum::{
    extract::Form,
    response::{IntoResponse, Redirect},
};
use serde::Deserialize;
use urlencoding;

use crate::{
    atproto::{
        lexicon::{
            community::lexicon::calendar::rsvp::{
                Rsvp as CommunityRsvpLexicon, RsvpStatus as CommunityRsvpStatusLexicon,
                NSID as COMMUNITY_RSVP_NSID,
            },
            events::smokesignal::calendar::rsvp::{
                Rsvp as SmokesignalRsvpLexicon, NSID as SMOKESIGNAL_RSVP_NSID,
            },
        },
        uri::parse_aturi,
    },
    contextual_error,
    http::{
        context::{admin_template_context, AdminRequestContext},
        errors::{AdminImportRsvpError, CommonError, LoginError, WebError},
    },
    resolve::{parse_input, resolve_subject, InputType},
    select_template,
    storage::{event::rsvp_insert_with_metadata, handle::handle_warm_up},
};

#[derive(Deserialize)]
pub struct ImportRsvpForm {
    pub aturi: String,
}

pub async fn handle_admin_import_rsvp(
    admin_ctx: AdminRequestContext,
    Form(form): Form<ImportRsvpForm>,
) -> Result<impl IntoResponse, WebError> {
    // Admin access is already verified by the extractor
    let canonical_url = format!(
        "https://{}/admin/rsvps",
        admin_ctx.web_context.config.external_base
    );
    let default_context = admin_template_context(&admin_ctx, &canonical_url);

    let error_template = select_template!(false, false, admin_ctx.language);

    // Parse the AT-URI
    let aturi = form.aturi.trim();
    let (repository, collection, rkey) = match parse_aturi(aturi) {
        Ok(parsed) => parsed,
        Err(_err) => {
            return contextual_error!(
                admin_ctx.web_context,
                admin_ctx.language,
                error_template,
                default_context,
                CommonError::InvalidAtUri
            );
        }
    };

    // Determine RSVP type based on collection
    let rsvp_format = match collection.as_str() {
        collection if collection == COMMUNITY_RSVP_NSID => "community",
        collection if collection == SMOKESIGNAL_RSVP_NSID => "smokesignal",
        _ => {
            return contextual_error!(
                admin_ctx.web_context,
                admin_ctx.language,
                error_template,
                default_context,
                CommonError::UnsupportedEventType
            );
        }
    };

    // Resolve the DID for the repository
    let input_type = match parse_input(&repository) {
        Ok(input) => input,
        Err(_err) => {
            return contextual_error!(
                admin_ctx.web_context,
                admin_ctx.language,
                error_template,
                default_context,
                CommonError::FailedToParse
            );
        }
    };

    let did = match input_type {
        InputType::Handle(handle) => {
            match resolve_subject(
                &admin_ctx.web_context.http_client,
                &admin_ctx.web_context.dns_resolver,
                &handle,
            )
            .await
            {
                Ok(did) => did,
                Err(_err) => {
                    return contextual_error!(
                        admin_ctx.web_context,
                        admin_ctx.language,
                        error_template,
                        default_context,
                        CommonError::FailedToParse
                    );
                }
            }
        }
        InputType::Plc(did) | InputType::Web(did) => did,
    };

    // Get the DID document to find the PDS endpoint
    let did_doc = match crate::did::plc::query(
        &admin_ctx.web_context.http_client,
        &admin_ctx.web_context.config.plc_hostname,
        &did,
    )
    .await
    {
        Ok(doc) => doc,
        Err(_err) => {
            return contextual_error!(
                admin_ctx.web_context,
                admin_ctx.language,
                error_template,
                default_context,
                CommonError::FailedToParse
            );
        }
    };

    // Insert the handle if it doesn't exist
    if let Some(handle) = did_doc.primary_handle() {
        if let Some(pds) = did_doc.pds_endpoint() {
            if let Err(err) = handle_warm_up(&admin_ctx.web_context.pool, &did, handle, pds).await {
                tracing::warn!("Failed to insert handle: {}", err);
            }
        }
    }

    // Get the PDS endpoint
    let pds_endpoint = match did_doc.pds_endpoint() {
        Some(endpoint) => endpoint,
        None => {
            return contextual_error!(
                admin_ctx.web_context,
                admin_ctx.language,
                error_template,
                default_context,
                WebError::Login(LoginError::NoPDS)
            );
        }
    };

    // Construct the XRPC request to get the record
    let url = format!(
        "{}/xrpc/com.atproto.repo.getRecord?repo={}&collection={}&rkey={}",
        pds_endpoint, did, collection, rkey
    );

    let response = match admin_ctx.web_context.http_client.get(&url).send().await {
        Ok(resp) => resp,
        Err(_err) => {
            return contextual_error!(
                admin_ctx.web_context,
                admin_ctx.language,
                error_template,
                default_context,
                CommonError::RecordNotFound
            );
        }
    };

    // Generic RSVP response for JSON parsing
    #[derive(Deserialize)]
    struct RsvpResponse {
        cid: String,
        value: serde_json::Value,
    }

    // Parse the generic RSVP response first
    let generic_record = match response.json::<RsvpResponse>().await {
        Ok(record) => record,
        Err(_err) => {
            return contextual_error!(
                admin_ctx.web_context,
                admin_ctx.language,
                error_template,
                default_context,
                CommonError::FailedToParse
            );
        }
    };

    let cid = generic_record.cid;

    // Process the RSVP based on its format and store it in the database
    let result = if rsvp_format == "community" {
        // Parse as Community RSVP format
        let rsvp_value = match serde_json::from_value::<CommunityRsvpLexicon>(generic_record.value)
        {
            Ok(value) => value,
            Err(_err) => {
                return contextual_error!(
                    admin_ctx.web_context,
                    admin_ctx.language,
                    error_template,
                    default_context,
                    CommonError::FailedToParse
                );
            }
        };

        let (event_aturi, event_cid, status) = match &rsvp_value {
            CommunityRsvpLexicon::Current {
                subject, status, ..
            } => {
                let event_aturi = subject.uri.clone();
                let event_cid = subject.cid.clone();
                let status = match status {
                    CommunityRsvpStatusLexicon::Going => "going",
                    CommunityRsvpStatusLexicon::Interested => "interested",
                    CommunityRsvpStatusLexicon::NotGoing => "notgoing",
                };
                (event_aturi, event_cid, status)
            }
        };

        rsvp_insert_with_metadata(
            &admin_ctx.web_context.pool,
            crate::storage::event::RsvpInsertParams {
                aturi,
                cid: &cid,
                did: &did,
                lexicon: COMMUNITY_RSVP_NSID,
                record: &rsvp_value,
                event_aturi: &event_aturi,
                event_cid: &event_cid,
                status,
            },
        )
        .await
    } else {
        // Parse as Smokesignal RSVP format
        let rsvp_value =
            match serde_json::from_value::<SmokesignalRsvpLexicon>(generic_record.value) {
                Ok(value) => value,
                Err(_err) => {
                    return contextual_error!(
                        admin_ctx.web_context,
                        admin_ctx.language,
                        error_template,
                        default_context,
                        CommonError::FailedToParse
                    );
                }
            };

        // Extract event URI, CID, and status from Smokesignal RSVP
        let (event_aturi, event_cid, status) = match &rsvp_value {
            SmokesignalRsvpLexicon::Current {
                subject, status, ..
            } => {
                let event_aturi = subject.uri.clone();
                let event_cid = subject.cid.clone();
                let status = match status {
                crate::atproto::lexicon::events::smokesignal::calendar::rsvp::RsvpStatus::Going => "going",
                crate::atproto::lexicon::events::smokesignal::calendar::rsvp::RsvpStatus::Interested => "interested",
                crate::atproto::lexicon::events::smokesignal::calendar::rsvp::RsvpStatus::NotGoing => "notgoing",
            };
                (event_aturi, event_cid, status)
            }
        };

        // Call the generic function with extracted values
        rsvp_insert_with_metadata(
            &admin_ctx.web_context.pool,
            crate::storage::event::RsvpInsertParams {
                aturi,
                cid: &cid,
                did: &did,
                lexicon: SMOKESIGNAL_RSVP_NSID,
                record: &rsvp_value,
                event_aturi: &event_aturi,
                event_cid: &event_cid,
                status,
            },
        )
        .await
    };

    // Process the result of the database operation
    match result {
        Ok(_) => {
            // Redirect with success parameter
            let encoded_aturi = urlencoding::encode(aturi).to_string();
            Ok(Redirect::to(&format!(
                "/admin/rsvps?import_success=true&imported_aturi={}",
                encoded_aturi
            ))
            .into_response())
        }
        Err(err) => {
            contextual_error!(
                admin_ctx.web_context,
                admin_ctx.language,
                error_template,
                default_context,
                AdminImportRsvpError::InsertFailed(err.to_string())
            )
        }
    }
}
