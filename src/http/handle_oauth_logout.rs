use anyhow::Result;
use axum::{
    extract::State,
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::{cookie::Cookie, PrivateCookieJar};
use axum_htmx::{HxRedirect, HxRequest};
use axum_template::RenderHtml;
use http::StatusCode;
use minijinja::context as template_context;

use crate::http::{
    context::WebContext, errors::WebError, middleware_auth::AUTH_COOKIE_NAME,
    middleware_i18n::Language,
};

pub async fn handle_logout(
    State(web_context): State<WebContext>,
    Language(language): Language,
    HxRequest(hx_request): HxRequest,
    jar: PrivateCookieJar,
) -> Result<impl IntoResponse, WebError> {
    let updated_jar = jar.remove(Cookie::from(AUTH_COOKIE_NAME));

    if hx_request {
        let hx_redirect = HxRedirect::try_from("/");
        if let Err(err) = hx_redirect {
            tracing::error!("Failed to create HxLocation: {}", err);
            return Ok(RenderHtml(
                format!("alert.{}.partial.html", language.to_string().to_lowercase()),
                web_context.engine.clone(),
                template_context! { message => "Internal Server Error" },
            )
            .into_response());
        }
        let hx_redirect = hx_redirect.unwrap();
        Ok((StatusCode::OK, hx_redirect, "").into_response())
    } else {
        Ok((updated_jar, Redirect::to("/")).into_response())
    }
}
