use anyhow::Result;
use axum::{extract::State, response::IntoResponse, Json};
use serde::Serialize;

use super::{context::WebContext, errors::WebError};

#[derive(Serialize)]
struct AuthMetadata {
    client_id: String,
    dpop_bound_access_tokens: bool,
    application_type: &'static str,
    redirect_uris: Vec<String>,
    client_uri: String,
    grant_types: Vec<&'static str>,
    response_types: Vec<&'static str>,
    scope: &'static str,
    client_name: &'static str,
    token_endpoint_auth_method: &'static str,
    jwks_uri: String,
    logo_uri: String,
    tos_uri: &'static str,
    policy_uri: &'static str,
    subject_type: &'static str,
    token_endpoint_auth_signing_alg: &'static str,
}

pub async fn handle_oauth_metadata(
    State(web_context): State<WebContext>,
) -> Result<impl IntoResponse, WebError> {
    tracing::warn!("handle_oauth_metadata");

    let client_id = format!(
        "https://{}/oauth/client-metadata.json",
        web_context.config.external_base
    );
    let redirect_uris = vec![format!(
        "https://{}/oauth/callback",
        web_context.config.external_base
    )];
    let jwks_uri = format!(
        "https://{}/.well-known/jwks.json",
        web_context.config.external_base
    );

    let resp = AuthMetadata {
        application_type: "web",
        client_id,
        client_name: "Smoke Signal",
        client_uri: format!("https://{}", web_context.config.external_base),
        dpop_bound_access_tokens: true,
        grant_types: vec!["authorization_code", "refresh_token"],
        jwks_uri,
        logo_uri: format!(
            "https://{}/logo-160x160x.png",
            web_context.config.external_base
        ),
        policy_uri: "https://docs.smokesignal.events/docs/about/privacy/",
        redirect_uris,
        response_types: vec!["code"],
        scope: "atproto transition:generic",
        token_endpoint_auth_method: "private_key_jwt",
        token_endpoint_auth_signing_alg: "ES256",
        subject_type: "public",
        tos_uri: "https://docs.smokesignal.events/docs/about/terms/",
    };
    Ok(Json(resp))
}
