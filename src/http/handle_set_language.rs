use anyhow::Result;
use axum::{
    extract::State,
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::{
    cookie::{Cookie, CookieJar, SameSite},
    Cached, Form,
};
use minijinja::context as template_context;
use serde::Deserialize;
use std::{borrow::Cow, str::FromStr};
use unic_langid::LanguageIdentifier;

use crate::storage::handle::{handle_update_field, HandleField};

use super::{
    context::WebContext, errors::WebError, middleware_auth::Auth, middleware_i18n::COOKIE_LANG,
    templates::render_alert,
};

#[derive(Deserialize, Clone)]
pub struct LanguageForm {
    language: String,
}

#[tracing::instrument(skip_all, err)]
pub async fn handle_set_language(
    State(web_context): State<WebContext>,
    Cached(auth): Cached<Auth>,
    jar: CookieJar,
    Form(language_form): Form<LanguageForm>,
) -> Result<impl IntoResponse, WebError> {
    let default_context = template_context! {
        current_handle => auth.0,
        canonical_url => format!("https://{}/language", web_context.config.external_base),
    };

    let use_language = LanguageIdentifier::from_str(&language_form.language);
    if use_language.is_err() {
        return Ok(render_alert(
            web_context.engine.clone(),
            "en-us",
            "Invalid language",
            default_context,
        )
        .into_response());
    }

    let use_language = use_language.unwrap();

    let found = web_context
        .i18n_context
        .supported_languages
        .iter()
        .find(|lang| lang.matches(&use_language, true, false));
    if found.is_none() {
        return Ok(render_alert(
            web_context.engine.clone(),
            "en-us",
            "Invalid language",
            default_context,
        )
        .into_response());
    }
    let found = found.unwrap();

    if let Some(handle) = auth.0 {
        if let Err(err) = handle_update_field(
            &web_context.pool,
            &handle.did,
            HandleField::Language(Cow::Owned(found.to_string())),
        )
        .await
        {
            tracing::error!(error = ?err, "Failed to update language");
        }
    }

    let mut cookie = Cookie::new(COOKIE_LANG, found.to_string());
    cookie.set_path("/");
    cookie.set_http_only(true);
    cookie.set_secure(true);
    cookie.set_same_site(Some(SameSite::Lax));

    let updated_jar = jar.add(cookie);

    Ok((updated_jar, Redirect::to("/")).into_response())
}
