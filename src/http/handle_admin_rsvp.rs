use anyhow::Result;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
};
use axum_extra::extract::Cached;
use axum_template::RenderHtml;
use minijinja::context as template_context;
use serde::Deserialize;

use crate::{
    contextual_error,
    http::{
        context::WebContext, errors::WebError, middleware_auth::Auth, middleware_i18n::Language,
    },
    select_template,
    storage::event::rsvp_get,
};

#[derive(Deserialize)]
pub struct RsvpRecordQuery {
    pub aturi: String,
}

pub async fn handle_admin_rsvp(
    State(web_context): State<WebContext>,
    Language(language): Language,
    Cached(auth): Cached<Auth>,
    Query(query): Query<RsvpRecordQuery>,
) -> Result<impl IntoResponse, WebError> {
    let current_handle = auth.require_admin(&web_context.config)?;

    let default_context = template_context! {
        language => language.to_string(),
        current_handle,
        canonical_url => format!("https://{}/admin/rsvp", web_context.config.external_base),
    };

    let render_template = select_template!("admin_rsvp", false, false, language);
    let error_template = select_template!(false, false, language);

    // Fetch the RSVP
    let rsvp_result = rsvp_get(&web_context.pool, &query.aturi).await;
    if let Err(err) = rsvp_result {
        return contextual_error!(web_context, language, error_template, default_context, err);
    }
    let rsvp = rsvp_result.unwrap();

    // Convert the RSVP to a JSON string for display
    let rsvp_json = serde_json::to_string_pretty(&rsvp).unwrap_or_default();

    Ok(RenderHtml(
        &render_template,
        web_context.engine.clone(),
        template_context! { ..default_context, ..template_context! {
            rsvp,
            rsvp_json,
        }},
    )
    .into_response())
}
