use anyhow::Result;
use axum::{extract::State, response::IntoResponse};
use axum_extra::extract::Cached;
use axum_htmx::HxBoosted;
use axum_template::RenderHtml;
use minijinja::context as template_context;

use crate::http::{
    context::WebContext, errors::WebError, middleware_auth::Auth, middleware_i18n::Language,
};

pub async fn handle_view_feed(
    State(web_context): State<WebContext>,
    HxBoosted(hx_boosted): HxBoosted,
    Language(language): Language,
    Cached(auth): Cached<Auth>,
) -> Result<impl IntoResponse, WebError> {
    let render_template = if hx_boosted {
        format!("index.{}.bare.html", language.to_string().to_lowercase())
    } else {
        format!("index.{}.html", language.to_string().to_lowercase())
    };

    Ok(RenderHtml(
        &render_template,
        web_context.engine.clone(),
        template_context! {
            current_handle => auth.0,
            language => language.to_string(),
            canonical_url => format!("https://{}/", web_context.config.external_base),
        },
    )
    .into_response())
}
