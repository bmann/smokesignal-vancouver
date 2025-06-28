use anyhow::Result;
use axum::{
    extract::Form,
    response::{IntoResponse, Redirect},
};
use serde::Deserialize;

use crate::{
    atproto::{
        lexicon::{
            community::lexicon::calendar::event::Event as CommunityEventLexicon,
            events::smokesignal::calendar::event::{Event as SmokeSignalEvent, EventResponse},
        },
        uri::parse_aturi,
    },
    contextual_error,
    http::{
        context::{admin_template_context, AdminRequestContext},
        errors::{AdminImportEventError, CommonError, LoginError, WebError},
    },
    resolve::{parse_input, resolve_subject, InputType},
    select_template,
    storage::{event::event_insert_with_metadata, handle::handle_warm_up},
};

#[derive(Deserialize)]
pub struct ImportEventForm {
    pub aturi: String,
}

pub async fn handle_admin_import_event(
    admin_ctx: AdminRequestContext,
    Form(form): Form<ImportEventForm>,
) -> Result<impl IntoResponse, WebError> {
    // Admin access is already verified by the extractor
    let canonical_url = format!(
        "https://{}/admin/events",
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

    // Determine event type based on collection
    let event_format = match collection.as_str() {
        "events.smokesignal.calendar.event" => "smokesignal",
        "community.lexicon.calendar.event" => "community",
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

    // Parse the record based on collection type
    if event_format == "smokesignal" {
        // Handle SmokeSignal event format
        let record = match response.json::<EventResponse>().await {
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

        // Get name from SmokeSignal event format
        let name = match &record.value {
            SmokeSignalEvent::Current { name, .. } => name.clone(),
        };

        // Store event using the generic event_insert_with_metadata
        match event_insert_with_metadata(
            &admin_ctx.web_context.pool,
            aturi,
            &record.cid,
            &did,
            "events.smokesignal.calendar.event",
            &record.value,
            &name,
        )
        .await
        {
            Ok(_) => Ok(Redirect::to("/admin/events").into_response()),
            Err(err) => {
                contextual_error!(
                    admin_ctx.web_context,
                    admin_ctx.language,
                    error_template,
                    default_context,
                    AdminImportEventError::InsertFailed(err.to_string())
                )
            }
        }
    } else {
        // Handle Community Lexicon event format
        #[derive(serde::Deserialize)]
        struct CommunityResponse {
            cid: String,
            value: CommunityEventLexicon,
        }

        let record = match response.json::<CommunityResponse>().await {
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

        // Get name from Community event format
        let name = match &record.value {
            CommunityEventLexicon::Current { name, .. } => name.clone(),
        };

        // Store event using the generic event_insert_with_metadata
        match event_insert_with_metadata(
            &admin_ctx.web_context.pool,
            aturi,
            &record.cid,
            &did,
            "community.lexicon.calendar.event",
            &record.value,
            &name,
        )
        .await
        {
            Ok(_) => Ok(Redirect::to("/admin/events").into_response()),
            Err(err) => {
                contextual_error!(
                    admin_ctx.web_context,
                    admin_ctx.language,
                    error_template,
                    default_context,
                    AdminImportEventError::InsertFailed(err.to_string())
                )
            }
        }
    }
}
