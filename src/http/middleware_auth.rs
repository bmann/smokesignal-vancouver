use anyhow::Result;
use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
    response::Response,
};
use axum_extra::extract::PrivateCookieJar;
use base64::{engine::general_purpose, Engine as _};
use p256::{
    ecdsa::{signature::Signer, Signature, SigningKey},
    SecretKey,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument, trace};

use crate::{
    config::Config,
    encoding::ToBase64,
    http::context::WebContext,
    http::errors::{AuthMiddlewareError, WebSessionError},
    storage::handle::model::Handle,
    storage::oauth::model::OAuthSession,
    storage::oauth::web_session_lookup,
};

use super::errors::middleware_errors::MiddlewareAuthError;

pub const AUTH_COOKIE_NAME: &str = "session1";

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct WebSession {
    pub did: String,
    pub session_group: String,
}

impl TryFrom<String> for WebSession {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        serde_json::from_str(&value)
            .map_err(WebSessionError::DeserializeFailed)
            .map_err(Into::into)
    }
}

impl TryInto<String> for WebSession {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<String, Self::Error> {
        serde_json::to_string(&self)
            .map_err(WebSessionError::SerializeFailed)
            .map_err(Into::into)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DestinationClaims {
    #[serde(rename = "d")]
    pub destination: String,

    #[serde(rename = "n")]
    pub nonce: String,
}

#[derive(Clone)]
pub struct Auth(pub Option<Handle>, pub Option<OAuthSession>);

impl Auth {
    /// Requires authentication and redirects to login with a signed token containing the original destination
    ///
    /// This creates a redirect URL with a signed token containing the destination,
    /// which the login handler can verify and redirect back to after successful authentication.
    #[instrument(level = "debug", skip(self, secret_key), err)]
    pub fn require(
        &self,
        secret_key: &SecretKey,
        location: &str,
    ) -> Result<Handle, MiddlewareAuthError> {
        if let Some(handle) = &self.0 {
            trace!(did = %handle.did, "User authenticated");
            return Ok(handle.clone());
        }

        debug!(
            location,
            "Authentication required, creating signed redirect"
        );

        // Create claims with destination and random nonce
        let claims = DestinationClaims {
            destination: location.to_string(),
            nonce: ulid::Ulid::new().to_string(),
        };

        // Encode claims to base64
        let claims = claims.to_base64()?;
        let claim_content = claims.to_string();
        let encoded_json_bytes = general_purpose::URL_SAFE_NO_PAD.encode(claims.as_bytes());

        // Sign the encoded claims
        let signing_key = SigningKey::from(secret_key);
        let signature: Signature = signing_key
            .try_sign(encoded_json_bytes.as_bytes())
            .map_err(AuthMiddlewareError::SigningFailed)?;

        // Format the final destination with claims and signature
        let destination = format!(
            "{}.{}",
            claim_content,
            general_purpose::URL_SAFE_NO_PAD.encode(signature.to_bytes())
        );

        trace!(
            destination_length = destination.len(),
            "Created signed destination token"
        );
        Err(MiddlewareAuthError::AccessDenied(destination))
    }

    /// Simpler authentication check that just redirects to root path
    ///
    /// Use this when you don't need to return to the original page after login
    #[instrument(level = "debug", skip(self), err)]
    pub fn require_flat(&self) -> Result<Handle, MiddlewareAuthError> {
        if let Some(handle) = &self.0 {
            trace!(did = %handle.did, "User authenticated");
            return Ok(handle.clone());
        }

        debug!("Authentication required, redirecting to root");
        Err(MiddlewareAuthError::AccessDenied("/".to_string()))
    }

    /// Requires admin authentication
    ///
    /// Returns NotFound error instead of redirecting to login for security reasons
    #[instrument(level = "debug", skip(self, config), err)]
    pub fn require_admin(&self, config: &Config) -> Result<Handle, MiddlewareAuthError> {
        if let Some(handle) = &self.0 {
            if config.is_admin(&handle.did) {
                debug!(did = %handle.did, "Admin authenticated");
                return Ok(handle.clone());
            }
            debug!(did = %handle.did, "User not an admin");
        } else {
            debug!("No authentication found for admin check");
        }

        // Return NotFound instead of redirect for security reasons
        Err(MiddlewareAuthError::NotFound)
    }
}

impl<S> FromRequestParts<S> for Auth
where
    S: Send + Sync,
    WebContext: FromRef<S>,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, context: &S) -> Result<Self, Self::Rejection> {
        trace!("Extracting Auth from request");
        let web_context = WebContext::from_ref(context);

        let cookie_jar = PrivateCookieJar::from_headers(
            &parts.headers,
            web_context.config.http_cookie_key.as_ref().clone(),
        );

        let session = cookie_jar
            .get(AUTH_COOKIE_NAME)
            .map(|user_cookie| user_cookie.value().to_owned())
            .and_then(|inner_value| WebSession::try_from(inner_value).ok());

        if let Some(web_session) = session {
            trace!(?web_session.session_group, "Found session cookie");

            match web_session_lookup(
                &web_context.pool,
                &web_session.session_group,
                Some(&web_session.did),
            )
            .await
            {
                Ok(record) => {
                    debug!(?web_session.session_group, "Session validated");
                    return Ok(Self(Some(record.0), Some(record.1)));
                }
                Err(err) => {
                    debug!(?web_session.session_group, ?err, "Invalid session");
                    return Ok(Self(None, None));
                }
            };
        }

        trace!("No session cookie found");
        Ok(Self(None, None))
    }
}
