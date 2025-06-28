use anyhow::Result;
use axum::{extract::State, response::IntoResponse};
use axum_extra::extract::{Cached, Form};
use axum_htmx::HxBoosted;
use axum_template::RenderHtml;
use http::StatusCode;
use minijinja::context as template_context;
use serde::Deserialize;
use std::borrow::Cow;
use unic_langid::LanguageIdentifier;

use crate::{
    contextual_error,
    http::{
        context::WebContext, errors::WebError, middleware_auth::Auth, middleware_i18n::Language,
        timezones::supported_timezones,
    },
    select_template,
    storage::handle::{handle_for_did, handle_update_field, HandleField},
};

#[derive(Deserialize, Clone, Debug)]
pub struct TimezoneForm {
    timezone: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct LanguageForm {
    language: String,
}

pub async fn handle_settings(
    State(web_context): State<WebContext>,
    Language(language): Language,
    Cached(auth): Cached<Auth>,
    HxBoosted(hx_boosted): HxBoosted,
) -> Result<impl IntoResponse, WebError> {
    // Require authentication
    let current_handle = auth.require(&web_context.config.destination_key, "/settings")?;

    let default_context = template_context! {
        current_handle => current_handle.clone(),
        language => language.to_string(),
        canonical_url => format!("https://{}/settings", web_context.config.external_base),
    };

    let render_template = select_template!("settings", hx_boosted, false, language);

    // Get available timezones
    let (_, timezones) = supported_timezones(Some(&current_handle));

    // Get the list of supported languages
    let supported_languages = web_context
        .i18n_context
        .supported_languages
        .iter()
        .map(|lang| lang.to_string())
        .collect::<Vec<String>>();

    // Render the form
    Ok((
        StatusCode::OK,
        RenderHtml(
            &render_template,
            web_context.engine.clone(),
            template_context! {
                timezones => timezones,
                languages => supported_languages,
                ..default_context,
            },
        ),
    )
        .into_response())
}

#[tracing::instrument(skip_all, err)]
pub async fn handle_timezone_update(
    State(web_context): State<WebContext>,
    Language(language): Language,
    Cached(auth): Cached<Auth>,
    Form(timezone_form): Form<TimezoneForm>,
) -> Result<impl IntoResponse, WebError> {
    let current_handle = auth.require_flat()?;

    let default_context = template_context! {
        current_handle => current_handle.clone(),
        language => language.to_string(),
    };

    let error_template = select_template!(false, true, language);
    let render_template = format!("settings.{}.tz.html", language.to_string().to_lowercase());

    let (_, timezones) = supported_timezones(Some(&current_handle));

    if timezone_form.timezone.is_empty()
        || timezone_form.timezone == current_handle.tz
        || !timezones.contains(&timezone_form.timezone.as_str())
    {
        return contextual_error!(
            web_context,
            language,
            error_template,
            default_context,
            "error-xxx Invalid timezone"
        );
    }

    if let Err(err) = handle_update_field(
        &web_context.pool,
        &current_handle.did,
        HandleField::Timezone(Cow::Owned(timezone_form.timezone)),
    )
    .await
    {
        return contextual_error!(web_context, language, error_template, default_context, err);
    }

    let current_handle = match handle_for_did(&web_context.pool, &current_handle.did).await {
        Ok(value) => value,
        Err(err) => {
            return contextual_error!(web_context, language, error_template, default_context, err);
        }
    };

    Ok((
        StatusCode::OK,
        RenderHtml(
            &render_template,
            web_context.engine.clone(),
            template_context! {
                timezone_updated => true,
                timezones,
                current_handle,
                ..default_context
            },
        ),
    )
        .into_response())
}

#[tracing::instrument(skip_all, err)]
pub async fn handle_language_update(
    State(web_context): State<WebContext>,
    Language(language): Language,
    Cached(auth): Cached<Auth>,
    Form(language_form): Form<LanguageForm>,
) -> Result<impl IntoResponse, WebError> {
    let current_handle = auth.require_flat()?;

    let default_context = template_context! {
        current_handle => current_handle.clone(),
        language => language.to_string(),
    };

    let error_template = select_template!(false, true, language);
    let render_template = format!(
        "settings.{}.language.html",
        language.to_string().to_lowercase()
    );

    // Get the list of supported languages
    let supported_languages = web_context
        .i18n_context
        .supported_languages
        .iter()
        .map(|lang| lang.to_string())
        .collect::<Vec<String>>();

    if language_form.language.is_empty() || language_form.language == current_handle.language {
        return contextual_error!(
            web_context,
            language,
            error_template,
            default_context,
            "error-xxx Invalid language"
        );
    }

    let lang_id = match language_form.language.parse::<LanguageIdentifier>() {
        Ok(value) => value,
        Err(err) => {
            return contextual_error!(web_context, language, error_template, default_context, err);
        }
    };

    if !web_context
        .i18n_context
        .supported_languages
        .iter()
        .any(|l| l == &lang_id)
    {
        return contextual_error!(
            web_context,
            language,
            error_template,
            default_context,
            "error-xxx Invalid language"
        );
    }

    if let Err(err) = handle_update_field(
        &web_context.pool,
        &current_handle.did,
        HandleField::Language(Cow::Owned(language_form.language)),
    )
    .await
    {
        return contextual_error!(web_context, language, error_template, default_context, err);
    }

    let current_handle = match handle_for_did(&web_context.pool, &current_handle.did).await {
        Ok(value) => value,
        Err(err) => {
            return contextual_error!(web_context, language, error_template, default_context, err);
        }
    };

    Ok((
        StatusCode::OK,
        RenderHtml(
            &render_template,
            web_context.engine.clone(),
            template_context! {
                current_handle,
                language_updated => true,
                languages => supported_languages,
                ..default_context
            },
        ),
    )
        .into_response())
}
