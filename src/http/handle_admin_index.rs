use anyhow::Result;
use axum::response::IntoResponse;
use axum_template::RenderHtml;
use minijinja::context as template_context;

use crate::http::context::{admin_template_context, AdminRequestContext};

use super::errors::WebError;

pub async fn handle_admin_index(
    admin_ctx: AdminRequestContext,
) -> Result<impl IntoResponse, WebError> {
    // User is already verified as admin by the extractor
    let canonical_url = format!(
        "https://{}/admin",
        admin_ctx.web_context.config.external_base
    );

    Ok(RenderHtml(
        "admin.en-us.html",
        admin_ctx.web_context.engine.clone(),
        template_context! {
            ..admin_template_context(&admin_ctx, &canonical_url),
        },
    )
    .into_response())
}
