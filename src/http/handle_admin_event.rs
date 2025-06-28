use anyhow::Result;
use axum::{extract::Query, response::IntoResponse};
use axum_template::RenderHtml;
use minijinja::context as template_context;
use serde::Deserialize;

use crate::{
    contextual_error,
    http::{context::AdminRequestContext, errors::WebError},
    select_template,
    storage::event::event_get,
};

#[derive(Deserialize)]
pub struct EventRecordQuery {
    pub aturi: String,
}

pub async fn handle_admin_event(
    admin_ctx: AdminRequestContext,
    Query(query): Query<EventRecordQuery>,
) -> Result<impl IntoResponse, WebError> {
    let language = admin_ctx.language;
    let web_context = admin_ctx.web_context;

    let canonical_url = format!("https://{}/admin/event", web_context.config.external_base);

    // Create context with query parameters
    let context_with_aturi = template_context! {
        language => language.to_string(),
        current_handle => admin_ctx.admin_handle.clone(),
        canonical_url => canonical_url,
        aturi => query.aturi.clone()
    };

    let render_template = select_template!("admin_event", false, false, language);
    let error_template = select_template!(false, false, language);

    // Get the event record by AT-URI
    let event = event_get(&web_context.pool, &query.aturi).await;
    if let Err(err) = event {
        return contextual_error!(
            web_context,
            language.0,
            error_template,
            context_with_aturi,
            err
        );
    }
    let event = event.unwrap();

    // Also provide the full event object as JSON
    let event_json = serde_json::to_string_pretty(&event)
        .unwrap_or_else(|_| "Error formatting JSON".to_string());

    Ok(RenderHtml(
        &render_template,
        web_context.engine.clone(),
        template_context! {
            language => language.to_string(),
            current_handle => admin_ctx.admin_handle.clone(),
            canonical_url => canonical_url,
            aturi => query.aturi.clone(),
            event => event,
            event_json => event_json,
        },
    )
    .into_response())
}
