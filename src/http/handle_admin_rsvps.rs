use anyhow::Result;
use axum::{extract::Query, response::IntoResponse};
use axum_template::RenderHtml;
use minijinja::context as template_context;
use serde::Deserialize;

use crate::{
    contextual_error,
    http::{
        context::AdminRequestContext,
        errors::WebError,
        pagination::{Pagination, PaginationView},
    },
    select_template,
    storage::event::rsvp_list,
};

#[derive(Deserialize, Default)]
pub struct AdminRsvpsParams {
    #[serde(flatten)]
    pagination: Pagination,

    #[serde(default)]
    import_success: bool,
    imported_aturi: Option<String>,
}

pub async fn handle_admin_rsvps(
    admin_ctx: AdminRequestContext,
    Query(params): Query<AdminRsvpsParams>,
) -> Result<impl IntoResponse, WebError> {
    let language = admin_ctx.language;
    let web_context = admin_ctx.web_context;

    let import_success = params.import_success;
    let imported_aturi = params.imported_aturi;

    let canonical_url = format!("https://{}/admin/rsvps", web_context.config.external_base);

    // Create the context with all needed parameters
    let default_context = template_context! {
        language => language.to_string(),
        current_handle => admin_ctx.admin_handle.clone(),
        canonical_url => canonical_url,
        import_success => import_success,
        imported_aturi => imported_aturi,
    };

    let render_template = select_template!("admin_rsvps", false, false, language);
    let error_template = select_template!(false, false, language);

    let (page, page_size) = params.pagination.admin_clamped();

    let rsvps = rsvp_list(&web_context.pool, page, page_size).await;
    if let Err(err) = rsvps {
        return contextual_error!(
            web_context,
            language.0,
            error_template,
            default_context,
            err
        );
    }
    let (total_count, mut rsvps) = rsvps.unwrap();

    let params: Vec<(&str, &str)> = vec![];

    let pagination_view = PaginationView::new(page_size, rsvps.len() as i64, page, params);

    if rsvps.len() > page_size as usize {
        rsvps.truncate(page_size as usize);
    }

    Ok(RenderHtml(
        &render_template,
        web_context.engine.clone(),
        template_context! {
            language => language.to_string(),
            current_handle => admin_ctx.admin_handle.clone(),
            canonical_url => canonical_url,
            import_success => import_success,
            imported_aturi => imported_aturi,
            rsvps => rsvps,
            total_count => total_count,
            pagination => pagination_view,
        },
    )
    .into_response())
}
