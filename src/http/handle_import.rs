use axum::{
    extract::{Form, State},
    response::IntoResponse,
};
use axum_extra::extract::Cached;
use axum_htmx::{HxBoosted, HxRequest};
use axum_template::RenderHtml;
use http::StatusCode;
use minijinja::context as template_context;
use serde::Deserialize;

use crate::{
    atproto::{
        auth::SimpleOAuthSessionProvider,
        client::{ListRecordsParams, OAuthPdsClient},
        lexicon::{
            community::lexicon::calendar::{
                event::{Event as LexiconCommunityEvent, NSID as LEXICON_COMMUNITY_EVENT_NSID},
                rsvp::{
                    Rsvp as LexiconCommunityRsvp, RsvpStatus as LexiconCommunityRsvpStatus,
                    NSID as LEXICON_COMMUNITY_RSVP_NSID,
                },
            },
            events::smokesignal::calendar::{
                event::{Event as SmokeSignalEvent, NSID as SMOKESIGNAL_EVENT_NSID},
                rsvp::{
                    Rsvp as SmokeSignalRsvp, RsvpStatus as SmokeSignalRsvpStatus,
                    NSID as SMOKESIGNAL_RSVP_NSID,
                },
            },
        },
    },
    contextual_error,
    http::{
        context::WebContext,
        errors::{ImportError, WebError},
        middleware_auth::Auth,
        middleware_i18n::Language,
    },
    select_template,
    storage::event::{event_insert_with_metadata, rsvp_insert_with_metadata},
};

pub async fn handle_import(
    State(web_context): State<WebContext>,
    Language(language): Language,
    Cached(auth): Cached<Auth>,
    HxRequest(hx_request): HxRequest,
    HxBoosted(hx_boosted): HxBoosted,
) -> Result<impl IntoResponse, WebError> {
    let current_handle = auth.require(&web_context.config.destination_key, "/import")?;

    let default_context = template_context! {
        current_handle,
        language => language.to_string(),
        canonical_url => format!("https://{}/import", web_context.config.external_base),
    };

    let render_template = select_template!("import", hx_boosted, hx_request, language);

    Ok(RenderHtml(
        &render_template,
        web_context.engine.clone(),
        default_context,
    )
    .into_response())
}

#[derive(Debug, Deserialize)]
pub struct ImportForm {
    pub collection: Option<String>,
    pub cursor: Option<String>,
}

pub async fn handle_import_submit(
    State(web_context): State<WebContext>,
    Language(language): Language,
    Cached(auth): Cached<Auth>,
    HxRequest(hx_request): HxRequest,
    Form(import_form): Form<ImportForm>,
) -> Result<impl IntoResponse, WebError> {
    let current_handle = auth.require_flat()?;

    if !hx_request {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    }

    let render_template = select_template!("import", false, true, language);
    let error_template = select_template!(false, hx_request, language);

    let collections = [
        LEXICON_COMMUNITY_EVENT_NSID,
        LEXICON_COMMUNITY_RSVP_NSID,
        SMOKESIGNAL_EVENT_NSID,
        SMOKESIGNAL_RSVP_NSID,
    ];

    let collection = import_form.collection.unwrap_or(collections[0].to_string());
    let cursor = import_form.cursor;

    let client_auth: SimpleOAuthSessionProvider =
        SimpleOAuthSessionProvider::try_from(auth.1.unwrap())?;
    let client = OAuthPdsClient {
        http_client: &web_context.http_client,
        pds: &current_handle.pds,
    };

    const LIMIT: u32 = 20;

    // Set up list records parameters to fetch records
    let list_params = ListRecordsParams {
        repo: current_handle.did.clone(),
        collection: collection.clone(),
        limit: Some(LIMIT),
        cursor,
        reverse: None,
    };

    let render_context = match collection.as_str() {
        LEXICON_COMMUNITY_EVENT_NSID => {
            let results = client
                .list_records::<LexiconCommunityEvent>(&client_auth, &list_params)
                .await;
            match results {
                Ok(list_records) => {
                    let mut items = vec![];

                    for event_record in list_records.records {
                        let name = match &event_record.value {
                            LexiconCommunityEvent::Current { name, .. } => name.clone(),
                        };
                        let event_insert_resp = event_insert_with_metadata(
                            &web_context.pool,
                            &event_record.uri,
                            &event_record.cid,
                            &current_handle.did,
                            LEXICON_COMMUNITY_EVENT_NSID,
                            &event_record.value,
                            &name,
                        )
                        .await;

                        let is_ok = if let Err(err) = event_insert_resp {
                            tracing::error!(?err, "error inserting event");
                            false
                        } else {
                            true
                        };

                        items.push(format!("{} - {}", event_record.uri, is_ok));
                    }

                    let (collection, cursor) = if items.len() == LIMIT as usize {
                        (collection.to_string(), Some(list_records.cursor))
                    } else {
                        (LEXICON_COMMUNITY_RSVP_NSID.to_string(), None)
                    };

                    template_context! {
                        cursor,
                        items_paged => true,
                        items,
                        collection,
                        completed => false,
                    }
                }
                Err(err) => {
                    return contextual_error!(
                        web_context,
                        language,
                        error_template,
                        template_context! {},
                        ImportError::FailedToListCommunityEvents(err.to_string())
                    )
                }
            }
        }
        LEXICON_COMMUNITY_RSVP_NSID => {
            let results = client
                .list_records::<LexiconCommunityRsvp>(&client_auth, &list_params)
                .await;
            match results {
                Ok(list_records) => {
                    let mut items = vec![];

                    for rsvp_record in list_records.records {
                        let (event_uri, event_cid, status) = match &rsvp_record.value {
                            LexiconCommunityRsvp::Current {
                                subject, status, ..
                            } => {
                                let status_str = match status {
                                    LexiconCommunityRsvpStatus::Going => "going",
                                    LexiconCommunityRsvpStatus::Interested => "interested",
                                    LexiconCommunityRsvpStatus::NotGoing => "notgoing",
                                };
                                (subject.uri.clone(), subject.cid.clone(), status_str)
                            }
                        };

                        let rsvp_insert_resp = rsvp_insert_with_metadata(
                            &web_context.pool,
                            crate::storage::event::RsvpInsertParams {
                                aturi: &rsvp_record.uri,
                                cid: &rsvp_record.cid,
                                did: &current_handle.did,
                                lexicon: LEXICON_COMMUNITY_RSVP_NSID,
                                record: &rsvp_record.value,
                                event_aturi: &event_uri,
                                event_cid: &event_cid,
                                status,
                            },
                        )
                        .await;

                        let is_ok = if let Err(err) = rsvp_insert_resp {
                            tracing::error!(?err, "error inserting community RSVP");
                            false
                        } else {
                            true
                        };

                        items.push(format!("{} - {}", rsvp_record.uri, is_ok));
                    }

                    let (collection, cursor) = if items.len() == LIMIT as usize {
                        (collection.to_string(), Some(list_records.cursor))
                    } else {
                        (SMOKESIGNAL_EVENT_NSID.to_string(), None)
                    };

                    template_context! {
                        cursor,
                        items_paged => true,
                        items,
                        collection,
                        completed => false,
                    }
                }
                Err(err) => {
                    return contextual_error!(
                        web_context,
                        language,
                        error_template,
                        template_context! {},
                        ImportError::FailedToListCommunityRSVPs(err.to_string())
                    )
                }
            }
        }
        SMOKESIGNAL_EVENT_NSID => {
            let results = client
                .list_records::<SmokeSignalEvent>(&client_auth, &list_params)
                .await;
            match results {
                Ok(list_records) => {
                    let mut items = vec![];

                    for event_record in list_records.records {
                        let name = match &event_record.value {
                            SmokeSignalEvent::Current { name, .. } => name.clone(),
                        };
                        let event_insert_resp = event_insert_with_metadata(
                            &web_context.pool,
                            &event_record.uri,
                            &event_record.cid,
                            &current_handle.did,
                            SMOKESIGNAL_EVENT_NSID,
                            &event_record.value,
                            &name,
                        )
                        .await;

                        let is_ok = if let Err(err) = event_insert_resp {
                            tracing::error!(?err, "error inserting Smokesignal event");
                            false
                        } else {
                            true
                        };

                        items.push(format!("{} - {}", event_record.uri, is_ok));
                    }

                    let (collection, cursor) = if items.len() == LIMIT as usize {
                        (collection.to_string(), Some(list_records.cursor))
                    } else {
                        (SMOKESIGNAL_RSVP_NSID.to_string(), None)
                    };

                    template_context! {
                        cursor,
                        items_paged => true,
                        items,
                        collection,
                        completed => false,
                    }
                }
                Err(err) => {
                    return contextual_error!(
                        web_context,
                        language,
                        error_template,
                        template_context! {},
                        ImportError::FailedToListSmokesignalEvents(err.to_string())
                    )
                }
            }
        }
        SMOKESIGNAL_RSVP_NSID => {
            let results = client
                .list_records::<SmokeSignalRsvp>(&client_auth, &list_params)
                .await;
            match results {
                Ok(list_records) => {
                    let mut items = vec![];

                    for rsvp_record in list_records.records {
                        let (event_uri, event_cid, status) = match &rsvp_record.value {
                            SmokeSignalRsvp::Current {
                                subject, status, ..
                            } => {
                                let status_str = match status {
                                    SmokeSignalRsvpStatus::Going => "going",
                                    SmokeSignalRsvpStatus::Interested => "interested",
                                    SmokeSignalRsvpStatus::NotGoing => "notgoing",
                                };
                                (subject.uri.clone(), subject.cid.clone(), status_str)
                            }
                        };

                        let rsvp_insert_resp = rsvp_insert_with_metadata(
                            &web_context.pool,
                            crate::storage::event::RsvpInsertParams {
                                aturi: &rsvp_record.uri,
                                cid: &rsvp_record.cid,
                                did: &current_handle.did,
                                lexicon: SMOKESIGNAL_RSVP_NSID,
                                record: &rsvp_record.value,
                                event_aturi: &event_uri,
                                event_cid: &event_cid,
                                status,
                            },
                        )
                        .await;

                        let is_ok = if let Err(err) = rsvp_insert_resp {
                            tracing::error!(?err, "error inserting Smokesignal RSVP");
                            false
                        } else {
                            true
                        };

                        items.push(format!("{} - {}", rsvp_record.uri, is_ok));
                    }

                    let completed = items.len() == LIMIT as usize;

                    template_context! {
                        cursor => list_records.cursor,
                        items_paged => true,
                        items,
                        collection,
                        completed,
                    }
                }
                Err(err) => {
                    return contextual_error!(
                        web_context,
                        language,
                        error_template,
                        template_context! {},
                        ImportError::FailedToListSmokesignalRSVPs(err.to_string())
                    )
                }
            }
        }
        _ => {
            return contextual_error!(
                web_context,
                language,
                error_template,
                template_context! {},
                ImportError::UnsupportedCollectionType(collection.clone())
            )
        }
    };

    Ok(RenderHtml(
        &render_template,
        web_context.engine.clone(),
        template_context! {
            current_handle,
            language => language.to_string(),
            canonical_url => format!("https://{}/import", web_context.config.external_base),
            ..render_context,
        },
    )
    .into_response())
}
