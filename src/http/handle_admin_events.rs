use anyhow::Result;
use axum::{extract::Query, response::IntoResponse};
use axum_template::RenderHtml;
use minijinja::context as template_context;

use crate::{
    contextual_error,
    http::{
        context::AdminRequestContext,
        errors::WebError,
        pagination::{Pagination, PaginationView},
    },
    select_template,
    storage::event::event_list,
};

pub async fn handle_admin_events(
    admin_ctx: AdminRequestContext,
    pagination: Query<Pagination>,
) -> Result<impl IntoResponse, WebError> {
    let language = admin_ctx.language;
    let web_context = admin_ctx.web_context;

    let canonical_url = format!("https://{}/admin/events", web_context.config.external_base);
    let default_context = template_context! {
        language => language.to_string(),
        current_handle => admin_ctx.admin_handle.clone(),
        canonical_url => canonical_url,
    };

    let render_template = select_template!("admin_events", false, false, language);
    let error_template = select_template!(false, false, language);

    let (page, page_size) = pagination.admin_clamped();

    let events = event_list(&web_context.pool, page, page_size).await;
    if let Err(err) = events {
        return contextual_error!(
            web_context,
            language.0,
            error_template,
            default_context,
            err
        );
    }
    let (total_count, mut events) = events.unwrap();

    let params: Vec<(&str, &str)> = vec![];

    let pagination_view = PaginationView::new(page_size, events.len() as i64, page, params);

    if events.len() > page_size as usize {
        events.truncate(page_size as usize);
    }

    Ok(RenderHtml(
        &render_template,
        web_context.engine.clone(),
        template_context! {
            language => language.to_string(),
            current_handle => admin_ctx.admin_handle.clone(),
            canonical_url => canonical_url,
            events => events,
            total_count => total_count,
            pagination => pagination_view,
        },
    )
    .into_response())
}
