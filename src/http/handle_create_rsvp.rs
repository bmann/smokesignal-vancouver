use anyhow::Result;
use axum::{extract::State, response::IntoResponse};
use axum_extra::extract::{Cached, Form};
use axum_htmx::{HxBoosted, HxRequest};
use axum_template::RenderHtml;
use chrono::Utc;
use http::Method;
use metrohash::MetroHash64;
use minijinja::context as template_context;
use std::hash::Hasher;

use crate::{
    atproto::{
        auth::SimpleOAuthSessionProvider,
        client::{OAuthPdsClient, PutRecordRequest},
        lexicon::{
            com::atproto::repo::StrongRef,
            community::lexicon::calendar::rsvp::{Rsvp, RsvpStatus, NSID},
        },
    },
    contextual_error,
    http::{
        context::WebContext,
        errors::WebError,
        middleware_auth::Auth,
        middleware_i18n::Language,
        rsvp_form::{BuildRSVPForm, BuildRsvpContentState},
        utils::url_from_aturi,
    },
    select_template,
    storage::event::rsvp_insert,
};

pub async fn handle_create_rsvp(
    method: Method,
    State(web_context): State<WebContext>,
    Language(language): Language,
    Cached(auth): Cached<Auth>,
    HxRequest(hx_request): HxRequest,
    HxBoosted(hx_boosted): HxBoosted,
    Form(mut build_rsvp_form): Form<BuildRSVPForm>,
) -> Result<impl IntoResponse, WebError> {
    let current_handle = auth.require(&web_context.config.destination_key, "/rsvp")?;

    let default_context = template_context! {
        current_handle,
        language => language.to_string(),
        canonical_url => format!("https://{}/rsvp", web_context.config.external_base),
        hx_request,
        hx_boosted,
    };

    let render_template = select_template!("create_rsvp", hx_boosted, hx_request, language);
    let error_template = select_template!(hx_boosted, hx_request, language);

    if build_rsvp_form.build_state.is_none() {
        build_rsvp_form.build_state = Some(BuildRsvpContentState::default());
    }

    if method == Method::GET {
        #[cfg(debug_assertions)]
        {
            build_rsvp_form.status = Some("going".to_string());
        }

        build_rsvp_form.build_state = Some(BuildRsvpContentState::Selecting);

        return Ok(RenderHtml(
            &render_template,
            web_context.engine.clone(),
            template_context! { ..default_context, ..template_context! {
                build_rsvp_form,
            }},
        )
        .into_response());
    }

    match build_rsvp_form.build_state {
        Some(BuildRsvpContentState::Reset) => {
            build_rsvp_form.build_state = Some(BuildRsvpContentState::Selecting);
            build_rsvp_form.subject_aturi = None;
            build_rsvp_form.subject_cid = None;
            build_rsvp_form.status = Some("going".to_string());
        }
        Some(BuildRsvpContentState::Selecting) => {}
        Some(BuildRsvpContentState::Selected) => {
            build_rsvp_form
                .hydrate(
                    &web_context.pool,
                    &web_context.i18n_context.locales,
                    &language,
                )
                .await;

            let found_errors =
                build_rsvp_form.validate(&web_context.i18n_context.locales, &language);
            if found_errors {
                build_rsvp_form.build_state = Some(BuildRsvpContentState::Selecting);
            } else {
                build_rsvp_form.build_state = Some(BuildRsvpContentState::Selected);
            }
        }
        Some(BuildRsvpContentState::Review) => {
            build_rsvp_form
                .hydrate(
                    &web_context.pool,
                    &web_context.i18n_context.locales,
                    &language,
                )
                .await;

            let found_errors =
                build_rsvp_form.validate(&web_context.i18n_context.locales, &language);

            if !found_errors {
                let now = Utc::now();

                let client_auth: SimpleOAuthSessionProvider =
                    SimpleOAuthSessionProvider::try_from(auth.1.unwrap())?;

                let client = OAuthPdsClient {
                    http_client: &web_context.http_client,
                    pds: &current_handle.pds,
                };

                let subject = StrongRef {
                    uri: build_rsvp_form.subject_aturi.as_ref().unwrap().to_string(),
                    cid: build_rsvp_form.subject_cid.as_ref().unwrap().to_string(),
                };

                let status = match build_rsvp_form.status.as_ref().unwrap().as_str() {
                    "going" => RsvpStatus::Going,
                    "interested" => RsvpStatus::Interested,
                    "notgoing" => RsvpStatus::NotGoing,
                    _ => unreachable!(),
                };

                let mut h = MetroHash64::default();
                h.write(subject.uri.clone().as_bytes());

                let record_key = crockford::encode(h.finish());

                let the_record = Rsvp::Current {
                    created_at: now,
                    subject,
                    status,
                };

                let rsvp_record = PutRecordRequest {
                    repo: current_handle.did.clone(),
                    collection: NSID.to_string(),
                    validate: false,
                    record_key,
                    record: the_record.clone(),
                    swap_commit: None,
                    swap_record: None,
                };

                let put_record_result = client.put_record(&client_auth, rsvp_record).await;

                if let Err(err) = put_record_result {
                    return contextual_error!(
                        web_context,
                        language,
                        error_template,
                        default_context,
                        err
                    );
                }

                let create_record_result = put_record_result.unwrap();

                let rsvp_insert_result = rsvp_insert(
                    &web_context.pool,
                    &create_record_result.uri,
                    &create_record_result.cid,
                    &current_handle.did,
                    NSID,
                    &the_record,
                )
                .await;

                if let Err(err) = rsvp_insert_result {
                    return contextual_error!(
                        web_context,
                        language,
                        error_template,
                        default_context,
                        err
                    );
                }

                let event_url = url_from_aturi(
                    &web_context.config.external_base,
                    build_rsvp_form.subject_aturi.clone().unwrap().as_str(),
                )?;

                return Ok(RenderHtml(
                    &render_template,
                    web_context.engine.clone(),
                    template_context! { ..default_context, ..template_context! {
                        build_rsvp_form,
                        event_url,
                    }},
                )
                .into_response());
            }
        }
        None => unreachable!(),
    }

    Ok(RenderHtml(
        &render_template,
        web_context.engine.clone(),
        template_context! { ..default_context, ..template_context! {
            build_rsvp_form
        }},
    )
    .into_response())
}
