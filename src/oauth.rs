use dpop::DpopRetry;
use p256::SecretKey;
use rand::distributions::{Alphanumeric, DistString};
use reqwest_chain::ChainMiddleware;
use reqwest_middleware::ClientBuilder;
use std::time::Duration;

use crate::oauth_client_errors::OAuthClientError;
use crate::oauth_errors::{AuthServerValidationError, ResourceValidationError};
use model::{AuthorizationServer, OAuthProtectedResource, ParResponse, TokenResponse};

use crate::{
    jose::{
        jwt::{Claims, Header, JoseClaims},
        mint_token,
    },
    storage::{
        handle::model::Handle,
        oauth::model::{OAuthRequest, OAuthRequestState},
    },
};

const HTTP_CLIENT_TIMEOUT_SECS: u64 = 8;

pub async fn pds_resources(
    http_client: &reqwest::Client,
    pds: &str,
) -> Result<(OAuthProtectedResource, AuthorizationServer), OAuthClientError> {
    let protected_resource = oauth_protected_resource(http_client, pds).await?;

    let first_authorization_server = protected_resource
        .authorization_servers
        .first()
        .ok_or(OAuthClientError::InvalidOAuthProtectedResource)?;

    let authorization_server =
        oauth_authorization_server(http_client, first_authorization_server).await?;
    Ok((protected_resource, authorization_server))
}

pub async fn oauth_protected_resource(
    http_client: &reqwest::Client,
    pds: &str,
) -> Result<OAuthProtectedResource, OAuthClientError> {
    let destination = format!("{}/.well-known/oauth-protected-resource", pds);

    let resource: OAuthProtectedResource = http_client
        .get(destination)
        .timeout(Duration::from_secs(HTTP_CLIENT_TIMEOUT_SECS))
        .send()
        .await
        .map_err(OAuthClientError::OAuthProtectedResourceRequestFailed)?
        .json()
        .await
        .map_err(OAuthClientError::MalformedOAuthProtectedResourceResponse)?;

    if resource.resource != pds {
        return Err(OAuthClientError::InvalidOAuthProtectedResourceResponse(
            ResourceValidationError::ResourceMustMatchPds.into(),
        ));
    }

    if resource.authorization_servers.is_empty() {
        return Err(OAuthClientError::InvalidOAuthProtectedResourceResponse(
            ResourceValidationError::AuthorizationServersMustNotBeEmpty.into(),
        ));
    }

    Ok(resource)
}

#[tracing::instrument(skip(http_client), err)]
pub async fn oauth_authorization_server(
    http_client: &reqwest::Client,
    pds: &str,
) -> Result<AuthorizationServer, OAuthClientError> {
    let destination = format!("{}/.well-known/oauth-authorization-server", pds);

    let resource: AuthorizationServer = http_client
        .get(destination)
        .timeout(Duration::from_secs(HTTP_CLIENT_TIMEOUT_SECS))
        .send()
        .await
        .map_err(OAuthClientError::AuthorizationServerRequestFailed)?
        .json()
        .await
        .map_err(OAuthClientError::MalformedAuthorizationServerResponse)?;

    // All of this is going to change.

    if resource.issuer != pds {
        return Err(OAuthClientError::InvalidAuthorizationServerResponse(
            AuthServerValidationError::IssuerMustMatchPds.into(),
        ));
    }

    resource
        .response_types_supported
        .iter()
        .find(|&x| x == "code")
        .ok_or(OAuthClientError::InvalidAuthorizationServerResponse(
            AuthServerValidationError::ResponseTypesSupportMustIncludeCode.into(),
        ))?;

    resource
        .grant_types_supported
        .iter()
        .find(|&x| x == "authorization_code")
        .ok_or(OAuthClientError::InvalidAuthorizationServerResponse(
            AuthServerValidationError::GrantTypesSupportMustIncludeAuthorizationCode.into(),
        ))?;
    resource
        .grant_types_supported
        .iter()
        .find(|&x| x == "refresh_token")
        .ok_or(OAuthClientError::InvalidAuthorizationServerResponse(
            AuthServerValidationError::GrantTypesSupportMustIncludeRefreshToken.into(),
        ))?;
    resource
        .code_challenge_methods_supported
        .iter()
        .find(|&x| x == "S256")
        .ok_or(OAuthClientError::InvalidAuthorizationServerResponse(
            AuthServerValidationError::CodeChallengeMethodsSupportedMustIncludeS256.into(),
        ))?;
    resource
        .token_endpoint_auth_methods_supported
        .iter()
        .find(|&x| x == "none")
        .ok_or(OAuthClientError::InvalidAuthorizationServerResponse(
            AuthServerValidationError::TokenEndpointAuthMethodsSupportedMustIncludeNone.into(),
        ))?;
    resource
        .token_endpoint_auth_methods_supported
        .iter()
        .find(|&x| x == "private_key_jwt")
        .ok_or(OAuthClientError::InvalidAuthorizationServerResponse(
            AuthServerValidationError::TokenEndpointAuthMethodsSupportedMustIncludePrivateKeyJwt
                .into(),
        ))?;
    resource
        .token_endpoint_auth_signing_alg_values_supported
        .iter()
        .find(|&x| x == "ES256")
        .ok_or(OAuthClientError::InvalidAuthorizationServerResponse(
            AuthServerValidationError::TokenEndpointAuthSigningAlgValuesMustIncludeES256.into(),
        ))?;
    resource
        .scopes_supported
        .iter()
        .find(|&x| x == "atproto")
        .ok_or(OAuthClientError::InvalidAuthorizationServerResponse(
            AuthServerValidationError::ScopesSupportedMustIncludeAtProto.into(),
        ))?;
    resource
        .scopes_supported
        .iter()
        .find(|&x| x == "transition:generic")
        .ok_or(OAuthClientError::InvalidAuthorizationServerResponse(
            AuthServerValidationError::ScopesSupportedMustIncludeTransitionGeneric.into(),
        ))?;
    resource
        .dpop_signing_alg_values_supported
        .iter()
        .find(|&x| x == "ES256")
        .ok_or(OAuthClientError::InvalidAuthorizationServerResponse(
            AuthServerValidationError::DpopSigningAlgValuesSupportedMustIncludeES256.into(),
        ))?;

    if !(resource.authorization_response_iss_parameter_supported
        && resource.require_pushed_authorization_requests
        && resource.client_id_metadata_document_supported)
    {
        return Err(OAuthClientError::InvalidAuthorizationServerResponse(
            AuthServerValidationError::RequiredServerFeaturesMustBeSupported.into(),
        ));
    }

    Ok(resource)
}

pub async fn oauth_init(
    http_client: &reqwest::Client,
    external_url_base: &str,
    (secret_key_id, secret_key): (&str, SecretKey),
    dpop_secret_key: &SecretKey,
    handle: &str,
    authorization_server: &AuthorizationServer,
    oauth_request_state: &OAuthRequestState,
) -> Result<ParResponse, OAuthClientError> {
    let par_url = authorization_server
        .pushed_authorization_request_endpoint
        .clone();

    let redirect_uri = format!("https://{}/oauth/callback", external_url_base);
    let client_id = format!("https://{}/oauth/client-metadata.json", external_url_base);

    let scope = "atproto transition:generic".to_string();

    let client_assertion_header = Header {
        algorithm: Some("ES256".to_string()),
        key_id: Some(secret_key_id.to_string()),
        ..Default::default()
    };

    let client_assertion_jti = Alphanumeric.sample_string(&mut rand::thread_rng(), 30);
    let client_assertion_claims = Claims::new(JoseClaims {
        issuer: Some(client_id.clone()),
        subject: Some(client_id.clone()),
        audience: Some(authorization_server.issuer.clone()),
        json_web_token_id: Some(client_assertion_jti),
        issued_at: Some(chrono::Utc::now().timestamp() as u64),
        ..Default::default()
    });
    tracing::info!("client_assertion_claims: {:?}", client_assertion_claims);

    let client_assertion_token = mint_token(
        &secret_key,
        &client_assertion_header,
        &client_assertion_claims,
    )
    .map_err(|jose_err| OAuthClientError::MintTokenFailed(jose_err.into()))?;

    let now = chrono::Utc::now();
    let public_key = dpop_secret_key.public_key();

    let dpop_proof_header = Header {
        type_: Some("dpop+jwt".to_string()),
        algorithm: Some("ES256".to_string()),
        json_web_key: Some(public_key.to_jwk()),
        ..Default::default()
    };
    let dpop_proof_jti = Alphanumeric.sample_string(&mut rand::thread_rng(), 30);

    let dpop_proof_claim = Claims::new(JoseClaims {
        json_web_token_id: Some(dpop_proof_jti),
        http_method: Some("POST".to_string()),
        http_uri: Some(par_url.clone()),
        issued_at: Some(now.timestamp() as u64),
        expiration: Some((now + chrono::Duration::seconds(30)).timestamp() as u64),
        ..Default::default()
    });
    let dpop_proof_token = mint_token(dpop_secret_key, &dpop_proof_header, &dpop_proof_claim)
        .map_err(|jose_err| OAuthClientError::MintTokenFailed(jose_err.into()))?;

    let dpop_retry = DpopRetry::new(
        dpop_proof_header.clone(),
        dpop_proof_claim.clone(),
        dpop_secret_key.clone(),
    );

    let dpop_retry_client = ClientBuilder::new(http_client.clone())
        .with(ChainMiddleware::new(dpop_retry.clone()))
        .build();

    let params = [
        ("response_type", "code"),
        ("code_challenge", &oauth_request_state.code_challenge),
        ("code_challenge_method", "S256"),
        ("client_id", client_id.as_str()),
        ("state", oauth_request_state.state.as_str()),
        ("redirect_uri", redirect_uri.as_str()),
        ("scope", scope.as_str()),
        ("login_hint", handle),
        (
            "client_assertion_type",
            "urn:ietf:params:oauth:client-assertion-type:jwt-bearer",
        ),
        ("client_assertion", client_assertion_token.as_str()),
    ];

    tracing::warn!("params: {:?}", params);

    dpop_retry_client
        .post(par_url)
        .header("DPoP", dpop_proof_token.as_str())
        .form(&params)
        .timeout(Duration::from_secs(HTTP_CLIENT_TIMEOUT_SECS))
        .send()
        .await
        .map_err(OAuthClientError::PARMiddlewareRequestFailed)?
        .json()
        .await
        .map_err(OAuthClientError::MalformedPARResponse)
}

pub async fn oauth_complete(
    http_client: &reqwest::Client,
    external_url_base: &str,
    (secret_key_id, secret_key): (&str, SecretKey),
    callback_code: &str,
    oauth_request: &OAuthRequest,
    handle: &Handle,
    dpop_secret_key: &SecretKey,
) -> Result<TokenResponse, OAuthClientError> {
    let (_, authorization_server) = pds_resources(http_client, &handle.pds).await?;

    let client_assertion_header = Header {
        algorithm: Some("ES256".to_string()),
        key_id: Some(secret_key_id.to_string()),
        ..Default::default()
    };

    let client_id = format!("https://{}/oauth/client-metadata.json", external_url_base);
    let redirect_uri = format!("https://{}/oauth/callback", external_url_base);

    let client_assertion_jti = Alphanumeric.sample_string(&mut rand::thread_rng(), 30);
    let client_assertion_claims = Claims::new(JoseClaims {
        issuer: Some(client_id.clone()),
        subject: Some(client_id.clone()),
        audience: Some(authorization_server.issuer.clone()),
        json_web_token_id: Some(client_assertion_jti),
        issued_at: Some(chrono::Utc::now().timestamp() as u64),
        ..Default::default()
    });

    let client_assertion_token = mint_token(
        &secret_key,
        &client_assertion_header,
        &client_assertion_claims,
    )
    .map_err(|jose_err| OAuthClientError::MintTokenFailed(jose_err.into()))?;

    let params = [
        ("client_id", client_id.as_str()),
        ("redirect_uri", redirect_uri.as_str()),
        ("grant_type", "authorization_code"),
        ("code", callback_code),
        ("code_verifier", &oauth_request.pkce_verifier),
        (
            "client_assertion_type",
            "urn:ietf:params:oauth:client-assertion-type:jwt-bearer",
        ),
        ("client_assertion", client_assertion_token.as_str()),
    ];

    let public_key = dpop_secret_key.public_key();

    let token_endpoint = authorization_server.token_endpoint.clone();

    let now = chrono::Utc::now();

    let dpop_proof_header = Header {
        type_: Some("dpop+jwt".to_string()),
        algorithm: Some("ES256".to_string()),
        json_web_key: Some(public_key.to_jwk()),
        ..Default::default()
    };
    let dpop_proof_jti = Alphanumeric.sample_string(&mut rand::thread_rng(), 30);
    let dpop_proof_claim = Claims::new(JoseClaims {
        json_web_token_id: Some(dpop_proof_jti),
        http_method: Some("POST".to_string()),
        http_uri: Some(authorization_server.token_endpoint.clone()),
        issued_at: Some(now.timestamp() as u64),
        expiration: Some((now + chrono::Duration::seconds(30)).timestamp() as u64),
        ..Default::default()
    });
    let dpop_proof_token = mint_token(dpop_secret_key, &dpop_proof_header, &dpop_proof_claim)
        .map_err(|jose_err| OAuthClientError::MintTokenFailed(jose_err.into()))?;

    let dpop_retry = DpopRetry::new(
        dpop_proof_header.clone(),
        dpop_proof_claim.clone(),
        dpop_secret_key.clone(),
    );

    let dpop_retry_client = ClientBuilder::new(http_client.clone())
        .with(ChainMiddleware::new(dpop_retry.clone()))
        .build();

    dpop_retry_client
        .post(token_endpoint)
        .header("DPoP", dpop_proof_token.as_str())
        .form(&params)
        .timeout(Duration::from_secs(HTTP_CLIENT_TIMEOUT_SECS))
        .send()
        .await
        .map_err(OAuthClientError::TokenMiddlewareRequestFailed)?
        .json()
        .await
        .map_err(OAuthClientError::MalformedTokenResponse)
}

pub async fn client_oauth_refresh(
    http_client: &reqwest::Client,
    external_url_base: &str,
    (secret_key_id, secret_key): (&str, SecretKey),
    refresh_token: &str,
    handle: &Handle,
    dpop_secret_key: &SecretKey,
) -> Result<TokenResponse, OAuthClientError> {
    let (_, authorization_server) = pds_resources(http_client, &handle.pds).await?;

    let client_assertion_header = Header {
        algorithm: Some("ES256".to_string()),
        key_id: Some(secret_key_id.to_string()),
        ..Default::default()
    };

    let client_id = format!("https://{}/oauth/client-metadata.json", external_url_base);
    let redirect_uri = format!("https://{}/oauth/callback", external_url_base);

    let client_assertion_jti = Alphanumeric.sample_string(&mut rand::thread_rng(), 30);
    let client_assertion_claims = Claims::new(JoseClaims {
        issuer: Some(client_id.clone()),
        subject: Some(client_id.clone()),
        audience: Some(authorization_server.issuer.clone()),
        json_web_token_id: Some(client_assertion_jti),
        issued_at: Some(chrono::Utc::now().timestamp() as u64),
        ..Default::default()
    });

    let client_assertion_token = mint_token(
        &secret_key,
        &client_assertion_header,
        &client_assertion_claims,
    )
    .map_err(|jose_err| OAuthClientError::MintTokenFailed(jose_err.into()))?;

    let params = [
        ("client_id", client_id.as_str()),
        ("redirect_uri", redirect_uri.as_str()),
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        (
            "client_assertion_type",
            "urn:ietf:params:oauth:client-assertion-type:jwt-bearer",
        ),
        ("client_assertion", client_assertion_token.as_str()),
    ];

    tracing::info!("params: {:?}", params);

    let public_key = dpop_secret_key.public_key();

    let token_endpoint = authorization_server.token_endpoint.clone();

    let now = chrono::Utc::now();

    let dpop_proof_header = Header {
        type_: Some("dpop+jwt".to_string()),
        algorithm: Some("ES256".to_string()),
        json_web_key: Some(public_key.to_jwk()),
        ..Default::default()
    };
    let dpop_proof_jti = Alphanumeric.sample_string(&mut rand::thread_rng(), 30);
    let dpop_proof_claim = Claims::new(JoseClaims {
        json_web_token_id: Some(dpop_proof_jti),
        http_method: Some("POST".to_string()),
        http_uri: Some(authorization_server.token_endpoint.clone()),
        issued_at: Some(now.timestamp() as u64),
        expiration: Some((now + chrono::Duration::seconds(30)).timestamp() as u64),
        ..Default::default()
    });
    let dpop_proof_token = mint_token(dpop_secret_key, &dpop_proof_header, &dpop_proof_claim)
        .map_err(|jose_err| OAuthClientError::MintTokenFailed(jose_err.into()))?;

    let dpop_retry = DpopRetry::new(
        dpop_proof_header.clone(),
        dpop_proof_claim.clone(),
        dpop_secret_key.clone(),
    );

    let dpop_retry_client = ClientBuilder::new(http_client.clone())
        .with(ChainMiddleware::new(dpop_retry.clone()))
        .build();

    dpop_retry_client
        .post(token_endpoint)
        .header("DPoP", dpop_proof_token.as_str())
        .form(&params)
        .timeout(Duration::from_secs(HTTP_CLIENT_TIMEOUT_SECS))
        .send()
        .await
        .map_err(OAuthClientError::TokenMiddlewareRequestFailed)?
        .json()
        .await
        .map_err(OAuthClientError::MalformedTokenResponse)
}

pub mod dpop {
    use p256::SecretKey;
    use reqwest::header::HeaderValue;
    use reqwest_chain::Chainer;
    use serde::Deserialize;

    use crate::{
        jose::{
            jwt::{Claims, Header},
            mint_token,
        },
        jose_errors::JoseError,
    };

    #[derive(Clone, Debug, Deserialize)]
    pub struct SimpleError {
        pub error: Option<String>,
        pub error_description: Option<String>,
        pub message: Option<String>,
    }

    impl std::fmt::Display for SimpleError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            if let Some(value) = &self.error {
                write!(f, "{}", value)
            } else if let Some(value) = &self.message {
                write!(f, "{}", value)
            } else if let Some(value) = &self.error_description {
                write!(f, "{}", value)
            } else {
                write!(f, "unknown")
            }
        }
    }

    #[derive(Clone)]
    pub struct DpopRetry {
        pub header: Header,
        pub claims: Claims,
        pub secret: SecretKey,
    }

    impl DpopRetry {
        pub fn new(header: Header, claims: Claims, secret: SecretKey) -> Self {
            DpopRetry {
                header,
                claims,
                secret,
            }
        }
    }

    #[async_trait::async_trait]
    impl Chainer for DpopRetry {
        type State = ();

        async fn chain(
            &self,
            result: Result<reqwest::Response, reqwest_middleware::Error>,
            _state: &mut Self::State,
            request: &mut reqwest::Request,
        ) -> Result<Option<reqwest::Response>, reqwest_middleware::Error> {
            let response = result?;

            let status_code = response.status();

            if status_code != 400 && status_code != 401 {
                return Ok(Some(response));
            };

            let headers = response.headers().clone();

            let simple_error = response.json::<SimpleError>().await;
            if simple_error.is_err() {
                return Err(reqwest_middleware::Error::Middleware(
                    JoseError::UnableToParseSimpleError.into(),
                ));
            }

            let simple_error = simple_error.unwrap();

            tracing::error!("dpop error: {:?}", simple_error);

            let is_use_dpop_nonce_error = simple_error
                .clone()
                .error
                .is_some_and(|error_value| error_value == "use_dpop_nonce");

            if !is_use_dpop_nonce_error {
                return Err(reqwest_middleware::Error::Middleware(
                    JoseError::UnexpectedError(simple_error.to_string()).into(),
                ));
            }

            let dpop_header = headers.get("DPoP-Nonce");

            if dpop_header.is_none() {
                return Err(reqwest_middleware::Error::Middleware(
                    JoseError::MissingDpopHeader.into(),
                ));
            }

            let new_dpop_header = dpop_header.unwrap().to_str().map_err(|dpop_header_err| {
                reqwest_middleware::Error::Middleware(
                    JoseError::UnableToParseDpopHeader(dpop_header_err.to_string()).into(),
                )
            })?;

            let dpop_proof_header = self.header.clone();
            let mut dpop_proof_claim = self.claims.clone();
            dpop_proof_claim
                .private
                .insert("nonce".to_string(), new_dpop_header.to_string().into());

            let dpop_proof_token = mint_token(&self.secret, &dpop_proof_header, &dpop_proof_claim)
                .map_err(|dpop_proof_token_err| {
                    reqwest_middleware::Error::Middleware(
                        JoseError::UnableToMintDpopProofToken(dpop_proof_token_err.to_string())
                            .into(),
                    )
                })?;

            request.headers_mut().insert(
                "DPoP",
                HeaderValue::from_str(&dpop_proof_token).expect("invalid header value"),
            );
            Ok(None)
        }
    }
}

pub mod model {
    use serde::Deserialize;

    #[derive(Clone, Deserialize)]
    pub struct OAuthProtectedResource {
        pub resource: String,
        pub authorization_servers: Vec<String>,
        pub scopes_supported: Vec<String>,
        pub bearer_methods_supported: Vec<String>,
    }

    #[derive(Clone, Deserialize, Default, Debug)]
    pub struct AuthorizationServer {
        pub introspection_endpoint: String,
        pub authorization_endpoint: String,
        pub authorization_response_iss_parameter_supported: bool,
        pub client_id_metadata_document_supported: bool,
        pub code_challenge_methods_supported: Vec<String>,
        pub dpop_signing_alg_values_supported: Vec<String>,
        pub grant_types_supported: Vec<String>,
        pub issuer: String,
        pub pushed_authorization_request_endpoint: String,
        pub request_parameter_supported: bool,
        pub require_pushed_authorization_requests: bool,
        pub response_types_supported: Vec<String>,
        pub scopes_supported: Vec<String>,
        pub token_endpoint_auth_methods_supported: Vec<String>,
        pub token_endpoint_auth_signing_alg_values_supported: Vec<String>,
        pub token_endpoint: String,
    }

    #[derive(Clone, Deserialize)]
    pub struct ParResponse {
        pub request_uri: String,
        pub expires_in: u64,
    }

    #[derive(Clone, Deserialize)]
    pub struct TokenResponse {
        pub access_token: String,
        pub token_type: String,
        pub refresh_token: String,
        pub scope: String,
        pub expires_in: u32,
        pub sub: String,
    }
}

// This errors module is now deprecated.
// Use crate::oauth_client_errors::OAuthClientError instead.
pub mod errors {
    pub use crate::oauth_client_errors::OAuthClientError;
}