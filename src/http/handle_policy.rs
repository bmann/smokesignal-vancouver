use anyhow::Result;
use axum::{extract::State, response::IntoResponse};
use axum_extra::extract::Cached;
use axum_htmx::HxBoosted;
use axum_template::RenderHtml;

use minijinja::context as template_context;

use crate::{
    http::{
        context::WebContext, errors::WebError, middleware_auth::Auth, middleware_i18n::Language,
    },
    select_template,
};

pub async fn handle_privacy_policy(
    State(web_context): State<WebContext>,
    HxBoosted(hx_boosted): HxBoosted,
    Language(language): Language,
    Cached(auth): Cached<Auth>,
) -> Result<impl IntoResponse, WebError> {
    let render_template = select_template!("privacy-policy", hx_boosted, false, language);
    Ok((
        http::StatusCode::OK,
        RenderHtml(
            &render_template,
            web_context.engine.clone(),
            template_context! {
                current_handle => auth.0,
                language => language.to_string(),
                canonical_url => format!("https://{}/privacy-policy", web_context.config.external_base),
            },
        ),
    )
        .into_response())
}

pub async fn handle_terms_of_service(
    State(web_context): State<WebContext>,
    HxBoosted(hx_boosted): HxBoosted,
    Language(language): Language,
    Cached(auth): Cached<Auth>,
) -> Result<impl IntoResponse, WebError> {
    let render_template = select_template!("terms-of-service", hx_boosted, false, language);
    Ok((
        http::StatusCode::OK,
        RenderHtml(
            &render_template,
            web_context.engine.clone(),
            template_context! {
                current_handle => auth.0,
                language => language.to_string(),
                canonical_url => format!("https://{}/terms-of-service", web_context.config.external_base),
            },
        ),
    )
        .into_response())
}

pub async fn handle_cookie_policy(
    State(web_context): State<WebContext>,
    HxBoosted(hx_boosted): HxBoosted,
    Language(language): Language,
    Cached(auth): Cached<Auth>,
) -> Result<impl IntoResponse, WebError> {
    let render_template = select_template!("cookie-policy", hx_boosted, false, language);
    Ok((
        http::StatusCode::OK,
        RenderHtml(
            &render_template,
            web_context.engine.clone(),
            template_context! {
                current_handle => auth.0,
                language => language.to_string(),
                canonical_url => format!("https://{}/cookie-policy", web_context.config.external_base),
            },
        ),
    )
        .into_response())
}

pub async fn handle_acknowledgement(
    State(web_context): State<WebContext>,
    HxBoosted(hx_boosted): HxBoosted,
    Language(language): Language,
    Cached(auth): Cached<Auth>,
) -> Result<impl IntoResponse, WebError> {
    let render_template = select_template!("acknowledgement", hx_boosted, false, language);
    Ok((
        http::StatusCode::OK,
        RenderHtml(
            &render_template,
            web_context.engine.clone(),
            template_context! {
                current_handle => auth.0,
                language => language.to_string(),
                canonical_url => format!("https://{}/acknowledgement", web_context.config.external_base),
            },
        ),
    )
        .into_response())
}
