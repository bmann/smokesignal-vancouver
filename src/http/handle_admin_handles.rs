use anyhow::Result;
use axum::{
    extract::{Path, Query},
    response::{IntoResponse, Redirect},
};
use axum_htmx::{HxRedirect, HxRequest};
use axum_template::RenderHtml;
use http::StatusCode;
use minijinja::context as template_context;

use crate::{
    contextual_error,
    http::{
        context::{admin_template_context, AdminRequestContext},
        errors::WebError,
        pagination::{Pagination, PaginationView},
    },
    select_template,
    storage::handle::{handle_list, handle_nuke},
};

pub async fn handle_admin_handles(
    admin_ctx: AdminRequestContext,
    pagination: Query<Pagination>,
) -> Result<impl IntoResponse, WebError> {
    let canonical_url = format!(
        "https://{}/admin/handles",
        admin_ctx.web_context.config.external_base
    );
    let default_context = admin_template_context(&admin_ctx, &canonical_url);

    let render_template = select_template!("admin_handles", false, false, admin_ctx.language);
    let error_template = select_template!(false, false, admin_ctx.language);

    let (page, page_size) = pagination.admin_clamped();

    let handles = handle_list(&admin_ctx.web_context.pool, page, page_size).await;
    if let Err(err) = handles {
        return contextual_error!(
            admin_ctx.web_context,
            admin_ctx.language,
            error_template,
            default_context,
            err
        );
    }
    let (total_count, mut handles) = handles.unwrap();

    let params: Vec<(&str, &str)> = vec![];

    let pagination_view = PaginationView::new(page_size, handles.len() as i64, page, params);

    if handles.len() > page_size as usize {
        handles.truncate(page_size as usize);
    }

    Ok(RenderHtml(
        &render_template,
        admin_ctx.web_context.engine.clone(),
        template_context! { ..default_context, ..template_context! {
            handles,
            total_count,
            pagination => pagination_view,
        }},
    )
    .into_response())
}

pub async fn handle_admin_nuke_identity(
    admin_ctx: AdminRequestContext,
    HxRequest(hx_request): HxRequest,
    Path(did): Path<String>,
) -> Result<impl IntoResponse, WebError> {
    let error_template = select_template!(false, false, admin_ctx.language);

    if did == admin_ctx.admin_handle.did {
        return contextual_error!(
            admin_ctx.web_context,
            admin_ctx.language,
            error_template,
            template_context! {
                message => "You cannot nuke your own identity."
            },
            "You cannot nuke your own identity."
        );
    }

    if let Err(err) = handle_nuke(
        &admin_ctx.web_context.pool,
        &did,
        &admin_ctx.admin_handle.did,
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

    if hx_request {
        let hx_redirect = HxRedirect::try_from("/admin/handles");
        if let Err(err) = hx_redirect {
            return contextual_error!(
                admin_ctx.web_context,
                admin_ctx.language,
                error_template,
                template_context! {},
                err
            );
        }
        let hx_redirect = hx_redirect.unwrap();
        Ok((StatusCode::OK, hx_redirect, "").into_response())
    } else {
        Ok(Redirect::to("/admin/handles").into_response())
    }
}
