use anyhow::Result;
use axum::response::Redirect;
use axum::{extract::State, response::IntoResponse};
use axum_extra::extract::{Cached, Form, Query};
use axum_htmx::{HxBoosted, HxRedirect, HxRequest};
use axum_template::RenderHtml;
use base64::{engine::general_purpose, Engine as _};
use http::StatusCode;
use minijinja::context as template_context;
use p256::SecretKey;
use rand::{distributions::Alphanumeric, Rng};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::borrow::Cow;

use crate::{
    contextual_error,
    did::{plc::query as plc_query, web::query as web_query},
    http::{
        context::WebContext, errors::LoginError, errors::WebError, middleware_auth::Auth,
        middleware_i18n::Language, utils::stringify,
    },
    jose,
    oauth::{oauth_init, pds_resources},
    resolve::{parse_input, resolve_subject, InputType},
    select_template,
    storage::{
        denylist::denylist_exists,
        handle::handle_warm_up,
        oauth::{model::OAuthRequestState, oauth_request_insert},
    },
};

#[derive(Deserialize)]
pub struct OAuthLoginForm {
    pub handle: Option<String>,
    pub destination: Option<String>,
}

#[derive(Deserialize)]
pub struct Destination {
    pub destination: Option<String>,
}

pub async fn handle_oauth_login(
    State(web_context): State<WebContext>,
    Language(language): Language,
    Cached(auth): Cached<Auth>,
    HxRequest(hx_request): HxRequest,
    HxBoosted(hx_boosted): HxBoosted,
    Query(destination): Query<Destination>,
    Form(login_form): Form<OAuthLoginForm>,
) -> Result<impl IntoResponse, WebError> {
    let default_context = template_context! {
        current_handle => auth.0,
        language => language.to_string(),
        canonical_url => format!("https://{}/oauth/login", web_context.config.external_base),
        destination => destination.destination,
    };

    let render_template = select_template!("login", hx_boosted, hx_request, language);
    let error_template = select_template!(hx_boosted, hx_request, language);

    if let Some(subject) = login_form.handle {
        let resolved_did = resolve_subject(
            &web_context.http_client,
            &web_context.dns_resolver,
            &subject,
        )
        .await;

        if let Err(err) = resolved_did {
            return contextual_error!(
                web_context,
                language,
                render_template,
                template_context! { ..default_context, ..template_context! {
                    handle_error => true,
                    handle_input => subject,
                }},
                err
            );
        }

        let resolved_did = resolved_did.unwrap();

        let query_results = match parse_input(&resolved_did) {
            Ok(InputType::Plc(did)) => {
                plc_query(
                    &web_context.http_client,
                    &web_context.config.plc_hostname,
                    &did,
                )
                .await
            }
            Ok(InputType::Web(did)) => web_query(&web_context.http_client, &did).await,
            _ => Err(LoginError::NoHandle.into()),
        };

        let did_document = match query_results {
            Ok(value) => value,
            Err(err) => {
                return contextual_error!(
                    web_context,
                    language,
                    render_template,
                    template_context! { ..default_context, ..template_context! {
                        handle_error => true,
                        handle_input => subject,
                    }},
                    err
                );
            }
        };

        let mut lookup_values: Vec<&str> = vec![&resolved_did, &did_document.id];
        if let Some(pds) = did_document.pds_endpoint() {
            lookup_values.push(pds);
        }

        let handle_denied = match denylist_exists(&web_context.pool, &lookup_values).await {
            Ok(value) => value,
            Err(err) => {
                return contextual_error!(
                    web_context,
                    language,
                    error_template,
                    default_context,
                    err
                );
            }
        };

        if handle_denied {
            return contextual_error!(
                web_context,
                language,
                render_template,
                template_context! { ..default_context, ..template_context! {
                    handle_error => true,
                    handle_input => subject,
                }},
                "access-denied"
            );
        }

        let pds = match did_document.pds_endpoint() {
            Some(value) => value,
            None => {
                return contextual_error!(
                    web_context,
                    language,
                    render_template,
                    template_context! { ..default_context, ..template_context! {
                        handle_error => true,
                        handle_input => subject,
                    }},
                    LoginError::NoPDS
                );
            }
        };

        let primary_handle = match did_document.primary_handle() {
            Some(value) => value,
            None => {
                return contextual_error!(
                    web_context,
                    language,
                    render_template,
                    template_context! { ..default_context, ..template_context! {
                        handle_error => true,
                        handle_input => subject,
                    }},
                    LoginError::NoHandle
                );
            }
        };

        if let Err(err) =
            handle_warm_up(&web_context.pool, &did_document.id, primary_handle, pds).await
        {
            return contextual_error!(web_context, language, error_template, default_context, err);
        }

        let state: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(30)
            .map(char::from)
            .collect();
        let nonce: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(30)
            .map(char::from)
            .collect();
        let (pkce_verifier, code_challenge) = gen_pkce();

        let oauth_request_state = OAuthRequestState {
            state,
            nonce,
            code_challenge,
        };

        let pds_auth_resources = pds_resources(&web_context.http_client, pds).await;

        if let Err(err) = pds_auth_resources {
            return contextual_error!(web_context, language, error_template, default_context, err);
        }

        let (_, authorization_server) = pds_auth_resources.unwrap();
        tracing::info!(authorization_server = ?authorization_server, "resolved authorization server");

        let signing_key = web_context.config.select_oauth_signing_key();
        if let Err(err) = signing_key {
            return contextual_error!(web_context, language, error_template, default_context, err);
        }

        let (key_id, signing_key) = signing_key.unwrap();

        let dpop_jwk = jose::jwk::generate();
        let dpop_secret_key = SecretKey::from_jwk(&dpop_jwk.jwk);

        if let Err(err) = dpop_secret_key {
            return contextual_error!(web_context, language, error_template, default_context, err);
        }

        let dpop_secret_key = dpop_secret_key.unwrap();

        let par_response = oauth_init(
            &web_context.http_client,
            &web_context.config.external_base,
            (&key_id, signing_key),
            &dpop_secret_key,
            primary_handle,
            &authorization_server,
            &oauth_request_state,
        )
        .await;

        if let Err(err) = par_response {
            return contextual_error!(web_context, language, error_template, default_context, err);
        }

        let par_response = par_response.unwrap();

        let created_at = chrono::Utc::now();
        let expires_at = created_at + chrono::Duration::seconds(par_response.expires_in as i64);

        if let Err(err) = oauth_request_insert(
            &web_context.pool,
            crate::storage::oauth::OAuthRequestParams {
                oauth_state: Cow::Owned(oauth_request_state.state.clone()),
                issuer: Cow::Owned(authorization_server.issuer.clone()),
                did: Cow::Owned(did_document.id.clone()),
                nonce: Cow::Owned(oauth_request_state.nonce.clone()),
                pkce_verifier: Cow::Owned(pkce_verifier.clone()),
                secret_jwk_id: Cow::Owned(key_id.clone()),
                dpop_jwk: Some(dpop_jwk.clone()),
                destination: login_form.destination.clone().map(Cow::Owned),
                created_at,
                expires_at,
            },
        )
        .await
        {
            return contextual_error!(web_context, language, error_template, default_context, err);
        }

        let oauth_args = [
            (
                "request_uri".to_string(),
                urlencoding::encode(&par_response.request_uri).to_string(),
            ),
            (
                "client_id".to_string(),
                urlencoding::encode(&format!(
                    "https://{}/oauth/client-metadata.json",
                    web_context.config.external_base
                ))
                .to_string(),
            ),
        ];
        let oauth_args = oauth_args.iter().map(|(k, v)| (&**k, &**v)).collect();

        let destination = format!(
            "{}?{}",
            authorization_server.authorization_endpoint,
            stringify(oauth_args)
        );

        if hx_request {
            if let Ok(hx_redirect) = HxRedirect::try_from(destination.as_str()) {
                return Ok((StatusCode::OK, hx_redirect, "").into_response());
            }
        }

        return Ok(Redirect::temporary(destination.as_str()).into_response());
    }

    Ok(RenderHtml(
        &render_template,
        web_context.engine.clone(),
        template_context! { ..default_context, ..template_context! {
            destination => destination.destination,
        }},
    )
    .into_response())
}

pub fn gen_pkce() -> (String, String) {
    let token: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(100)
        .map(char::from)
        .collect();
    (token.clone(), pkce_challenge(&token))
}

pub fn pkce_challenge(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let result = hasher.finalize();

    general_purpose::URL_SAFE_NO_PAD.encode(result)
}
