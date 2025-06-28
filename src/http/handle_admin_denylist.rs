use anyhow::Result;
use axum::{
    extract::Query,
    response::{IntoResponse, Redirect},
    Form,
};
use axum_template::RenderHtml;
use minijinja::context as template_context;
use serde::Deserialize;
use std::borrow::Cow;

use crate::{
    contextual_error,
    http::{
        context::{admin_template_context, AdminRequestContext},
        errors::WebError,
        pagination::{Pagination, PaginationView},
    },
    select_template,
    storage::denylist::{denylist_add_or_update, denylist_list, denylist_remove},
};

#[derive(Debug, Deserialize)]
pub struct DenylistAddForm {
    pub subject: String,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct DenylistRemoveForm {
    pub subject: String,
}

pub async fn handle_admin_denylist(
    admin_ctx: AdminRequestContext,
    pagination: Query<Pagination>,
) -> Result<impl IntoResponse, WebError> {
    let canonical_url = format!(
        "https://{}/admin/denylist",
        admin_ctx.web_context.config.external_base
    );
    let default_context = admin_template_context(&admin_ctx, &canonical_url);

    let render_template = select_template!("admin_denylist", false, false, admin_ctx.language);
    let error_template = select_template!(false, false, admin_ctx.language);

    let (page, page_size) = pagination.admin_clamped();

    let denylist = denylist_list(&admin_ctx.web_context.pool, page, page_size).await;
    if let Err(err) = denylist {
        return contextual_error!(
            admin_ctx.web_context,
            admin_ctx.language,
            error_template,
            default_context,
            err
        );
    }
    let (total_count, mut entries) = denylist.unwrap();

    let params: Vec<(&str, &str)> = vec![];

    let pagination_view = PaginationView::new(page_size, entries.len() as i64, page, params);

    if entries.len() > page_size as usize {
        entries.truncate(page_size as usize);
    }

    Ok(RenderHtml(
        &render_template,
        admin_ctx.web_context.engine.clone(),
        template_context! { ..default_context, ..template_context! {
            entries,
            total_count,
            pagination => pagination_view,
        }},
    )
    .into_response())
}

pub async fn handle_admin_denylist_add(
    admin_ctx: AdminRequestContext,
    Form(form): Form<DenylistAddForm>,
) -> Result<impl IntoResponse, WebError> {
    let error_template = select_template!(false, false, admin_ctx.language);

    if let Err(err) = denylist_add_or_update(
        &admin_ctx.web_context.pool,
        Cow::Borrowed(&form.subject),
        Cow::Borrowed(&form.reason),
    )
    .await
    {
        return contextual_error!(
            admin_ctx.web_context,
            admin_ctx.language,
            error_template,
            template_context! {},
            err
        );
    }

    Ok(Redirect::to("/admin/denylist").into_response())
}

pub async fn handle_admin_denylist_remove(
    admin_ctx: AdminRequestContext,
    Form(form): Form<DenylistRemoveForm>,
) -> Result<impl IntoResponse, WebError> {
    let error_template = select_template!(false, false, admin_ctx.language);

    if let Err(err) = denylist_remove(&admin_ctx.web_context.pool, &form.subject).await {
        return contextual_error!(
            admin_ctx.web_context,
            admin_ctx.language,
            error_template,
            template_context! {},
            err
        );
    }

    Ok(Redirect::to("/admin/denylist").into_response())
}
