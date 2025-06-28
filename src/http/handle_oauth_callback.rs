use anyhow::Result;
use axum::{
    extract::State,
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::{
    cookie::{Cookie, SameSite},
    Form, PrivateCookieJar,
};
use deadpool_redis::redis::AsyncCommands as _;
use minijinja::context as template_context;
use p256::SecretKey;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

use crate::jose_errors::JwkError;
use crate::storage::errors::CacheError;

use crate::{
    contextual_error,
    oauth::oauth_complete,
    select_template,
    storage::{
        cache::OAUTH_REFRESH_QUEUE,
        handle::handle_for_did,
        oauth::{oauth_request_get, oauth_request_remove, oauth_session_insert},
    },
};

use super::{
    context::WebContext,
    errors::{LoginError, WebError},
    middleware_auth::{WebSession, AUTH_COOKIE_NAME},
    middleware_i18n::Language,
};

#[derive(Deserialize, Serialize)]
pub struct OAuthCallbackForm {
    pub state: Option<String>,
    pub iss: Option<String>,
    pub code: Option<String>,
}

pub async fn handle_oauth_callback(
    State(web_context): State<WebContext>,
    Language(language): Language,
    jar: PrivateCookieJar,
    Form(callback_form): Form<OAuthCallbackForm>,
) -> Result<impl IntoResponse, WebError> {
    let default_context = template_context! {
        language => language.to_string(),
        canonical_url => format!("https://{}/oauth/callback", web_context.config.external_base),
    };

    let error_template = select_template!(false, false, language);

    let (callback_code, callback_iss, callback_state) =
        match (callback_form.code, callback_form.iss, callback_form.state) {
            (Some(x), Some(y), Some(z)) => (x, y, z),
            _ => {
                return contextual_error!(
                    web_context,
                    language,
                    error_template,
                    default_context,
                    LoginError::OAuthCallbackIncomplete
                );
            }
        };

    let oauth_request = oauth_request_get(&web_context.pool, &callback_state).await;
    if let Err(err) = oauth_request {
        return contextual_error!(web_context, language, error_template, default_context, err);
    }

    let oauth_request = oauth_request.unwrap();

    if oauth_request.issuer != callback_iss {
        return contextual_error!(
            web_context,
            language,
            error_template,
            default_context,
            LoginError::OAuthIssuerMismatch
        );
    }

    let handle = handle_for_did(&web_context.pool, &oauth_request.did).await;
    if let Err(err) = handle {
        return contextual_error!(web_context, language, error_template, default_context, err);
    }

    let handle = handle.unwrap();

    let secret_signing_key = web_context
        .config
        .signing_keys
        .as_ref()
        .get(&oauth_request.secret_jwk_id)
        .cloned()
        .ok_or(JwkError::SecretKeyNotFound);

    if let Err(err) = secret_signing_key {
        return contextual_error!(web_context, language, error_template, default_context, err);
    }
    let secret_signing_key = secret_signing_key.unwrap();

    let dpop_secret_key = SecretKey::from_jwk(&oauth_request.dpop_jwk.jwk);

    if let Err(err) = dpop_secret_key {
        return contextual_error!(web_context, language, error_template, default_context, err);
    }
    let dpop_secret_key = dpop_secret_key.unwrap();

    let token_response = oauth_complete(
        &web_context.http_client,
        &web_context.config.external_base,
        (&oauth_request.secret_jwk_id, secret_signing_key),
        &callback_code,
        &oauth_request,
        &handle,
        &dpop_secret_key,
    )
    .await;
    if let Err(err) = token_response {
        return contextual_error!(web_context, language, error_template, default_context, err);
    }

    let token_response = token_response.unwrap();

    if let Err(err) = oauth_request_remove(&web_context.pool, &oauth_request.oauth_state).await {
        tracing::error!(error = ?err, "Unable to remove oauth_request");
    }

    let session_group = ulid::Ulid::new().to_string();
    let now = chrono::Utc::now();

    if let Err(err) = oauth_session_insert(
        &web_context.pool,
        crate::storage::oauth::OAuthSessionParams {
            session_group: Cow::Owned(session_group.clone()),
            access_token: Cow::Owned(token_response.access_token.clone()),
            did: Cow::Owned(token_response.sub.clone()),
            issuer: Cow::Owned(oauth_request.issuer.clone()),
            refresh_token: Cow::Owned(token_response.refresh_token.clone()),
            secret_jwk_id: Cow::Owned(oauth_request.secret_jwk_id.clone()),
            dpop_jwk: oauth_request.dpop_jwk.0.clone(),
            created_at: now,
            access_token_expires_at: now
                + chrono::Duration::seconds(token_response.expires_in as i64),
        },
    )
    .await
    {
        return contextual_error!(web_context, language, error_template, default_context, err);
    }

    {
        let mut conn = web_context
            .cache_pool
            .get()
            .await
            .map_err(CacheError::FailedToGetConnection)?;

        let modified_expires_at = ((token_response.expires_in as f64) * 0.8).round() as i64;
        let refresh_at = (now + chrono::Duration::seconds(modified_expires_at)).timestamp_millis();

        let _: () = conn
            .zadd(OAUTH_REFRESH_QUEUE, &session_group, refresh_at)
            .await
            .map_err(CacheError::FailedToPlaceInRefreshQueue)?;
    }

    let cookie_value: String = WebSession {
        did: token_response.sub.clone(),
        session_group: session_group.clone(),
    }
    .try_into()?;

    let mut cookie = Cookie::new(AUTH_COOKIE_NAME, cookie_value);
    cookie.set_domain(web_context.config.external_base.clone());
    cookie.set_path("/");
    cookie.set_http_only(true);
    cookie.set_secure(true);
    cookie.set_max_age(Some(cookie::time::Duration::days(1)));
    cookie.set_same_site(Some(SameSite::Lax));

    let updated_jar = jar.add(cookie);

    let destination = match oauth_request.destination {
        Some(destination) => destination,
        None => "/".to_string(),
    };

    Ok((updated_jar, Redirect::to(&destination)).into_response())
}
