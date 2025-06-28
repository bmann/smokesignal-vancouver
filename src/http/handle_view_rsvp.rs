use anyhow::Result;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
};
use axum_extra::extract::Cached;
use axum_htmx::{HxBoosted, HxRequest};
use axum_template::RenderHtml;
use minijinja::context as template_context;
use serde::Deserialize;

use crate::{
    contextual_error,
    http::{
        context::WebContext,
        errors::{RSVPError, WebError},
        middleware_auth::Auth,
        middleware_i18n::Language,
    },
    select_template,
    storage::event::rsvp_get,
};

#[derive(Deserialize)]
pub struct RsvpQuery {
    pub aturi: Option<String>,
}

pub async fn handle_view_rsvp(
    State(web_context): State<WebContext>,
    HxBoosted(hx_boosted): HxBoosted,
    HxRequest(hx_request): HxRequest,
    Language(language): Language,
    Cached(auth): Cached<Auth>,
    query: Query<RsvpQuery>,
) -> Result<impl IntoResponse, WebError> {
    let current_handle = auth.0.clone();

    let default_context = template_context! {
        current_handle,
        language => language.to_string(),
        canonical_url => format!("https://{}/rsvps", web_context.config.external_base),
    };

    let render_template = select_template!("view_rsvp", hx_boosted, hx_request, language);
    let error_template = select_template!(hx_boosted, hx_request, language);

    // If ATURI is provided, try to fetch and display the RSVP
    let context = if let Some(aturi) = &query.aturi {
        match rsvp_get(&web_context.pool, aturi).await {
            Ok(Some(rsvp)) => {
                // RSVP found, add to context
                let rsvp_json = serde_json::to_string_pretty(&rsvp).unwrap_or_default();
                template_context! { ..default_context, ..template_context! {
                    aturi,
                    rsvp,
                    rsvp_json,
                }}
            }
            Ok(None) => {
                return contextual_error!(
                    web_context,
                    language,
                    error_template,
                    template_context! { ..default_context, ..template_context! {
                        aturi,
                    }},
                    RSVPError::NotFound
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
        }
    } else {
        // No ATURI provided
        default_context
    };

    Ok(RenderHtml(&render_template, web_context.engine.clone(), context).into_response())
}
