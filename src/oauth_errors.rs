use thiserror::Error;

/// Represents errors that can occur during OAuth resource validation.
///
/// These errors occur when validating the configuration of an OAuth resource server
/// against the requirements of the AT Protocol.
#[derive(Debug, Error)]
pub enum ResourceValidationError {
    /// Error when the resource server URI doesn't match the PDS URI.
    ///
    /// This error occurs when the resource server URI in the OAuth configuration
    /// does not match the expected Personal Data Server (PDS) URI, which is required
    /// for proper AT Protocol OAuth integration.
    #[error("error-oauth-resource-1 Resource must match PDS")]
    ResourceMustMatchPds,

    /// Error when the authorization servers list is empty.
    ///
    /// This error occurs when the OAuth resource configuration doesn't specify
    /// any authorization servers, which is required for AT Protocol OAuth flows.
    #[error("error-oauth-resource-2 Authorization servers must not be empty")]
    AuthorizationServersMustNotBeEmpty,
}

/// Represents errors that can occur during OAuth authorization server validation.
///
/// These errors occur when validating the configuration of an OAuth authorization server
/// against the requirements specified by the AT Protocol.
#[derive(Debug, Error)]
pub enum AuthServerValidationError {
    /// Error when the authorization server issuer doesn't match the PDS.
    ///
    /// This error occurs when the issuer URI in the OAuth authorization server metadata
    /// does not match the expected Personal Data Server (PDS) URI.
    #[error("error-oauth-auth-server-1 Issuer must match PDS")]
    IssuerMustMatchPds,

    /// Error when the 'code' response type is not supported.
    ///
    /// This error occurs when the authorization server doesn't support the 'code' response type,
    /// which is required for the authorization code grant flow in AT Protocol.
    #[error("error-oauth-auth-server-2 Response types supported must include 'code'")]
    ResponseTypesSupportMustIncludeCode,

    /// Error when the 'authorization_code' grant type is not supported.
    ///
    /// This error occurs when the authorization server doesn't support the 'authorization_code'
    /// grant type, which is required for the AT Protocol OAuth flow.
    #[error("error-oauth-auth-server-3 Grant types supported must include 'authorization_code'")]
    GrantTypesSupportMustIncludeAuthorizationCode,

    /// Error when the 'refresh_token' grant type is not supported.
    ///
    /// This error occurs when the authorization server doesn't support the 'refresh_token'
    /// grant type, which is required for maintaining long-term access in AT Protocol.
    #[error("error-oauth-auth-server-4 Grant types supported must include 'refresh_token'")]
    GrantTypesSupportMustIncludeRefreshToken,

    /// Error when the 'S256' code challenge method is not supported.
    ///
    /// This error occurs when the authorization server doesn't support the 'S256' code
    /// challenge method for PKCE, which is required for secure authorization code flow.
    #[error("error-oauth-auth-server-5 Code challenge methods supported must include 'S256'")]
    CodeChallengeMethodsSupportedMustIncludeS256,

    /// Error when the 'none' token endpoint auth method is not supported.
    ///
    /// This error occurs when the authorization server doesn't support the 'none'
    /// token endpoint authentication method, which is used for public clients.
    #[error("error-oauth-auth-server-6 Token endpoint auth methods supported must include 'none'")]
    TokenEndpointAuthMethodsSupportedMustIncludeNone,

    /// Error when the 'private_key_jwt' token endpoint auth method is not supported.
    ///
    /// This error occurs when the authorization server doesn't support the 'private_key_jwt'
    /// token endpoint authentication method, which is required for AT Protocol clients.
    #[error("error-oauth-auth-server-7 Token endpoint auth methods supported must include 'private_key_jwt'")]
    TokenEndpointAuthMethodsSupportedMustIncludePrivateKeyJwt,

    /// Error when the 'ES256' signing algorithm is not supported for token endpoint auth.
    ///
    /// This error occurs when the authorization server doesn't support the 'ES256' signing
    /// algorithm for token endpoint authentication, which is required for AT Protocol.
    #[error("error-oauth-auth-server-8 Token endpoint auth signing algorithm values must include 'ES256'")]
    TokenEndpointAuthSigningAlgValuesMustIncludeES256,

    /// Error when the 'atproto' scope is not supported.
    ///
    /// This error occurs when the authorization server doesn't support the 'atproto'
    /// scope, which is required for accessing AT Protocol resources.
    #[error("error-oauth-auth-server-9 Scopes supported must include 'atproto'")]
    ScopesSupportedMustIncludeAtProto,

    /// Error when the 'transition:generic' scope is not supported.
    ///
    /// This error occurs when the authorization server doesn't support the 'transition:generic'
    /// scope, which is required for transitional functionality in AT Protocol.
    #[error("error-oauth-auth-server-10 Scopes supported must include 'transition:generic'")]
    ScopesSupportedMustIncludeTransitionGeneric,

    /// Error when the 'ES256' DPoP signing algorithm is not supported.
    ///
    /// This error occurs when the authorization server doesn't support the 'ES256'
    /// signing algorithm for DPoP proofs, which is required for AT Protocol security.
    #[error(
        "error-oauth-auth-server-11 DPoP signing algorithm values supported must include 'ES256'"
    )]
    DpopSigningAlgValuesSupportedMustIncludeES256,

    /// Error when required server features are not supported.
    ///
    /// This error occurs when the authorization server doesn't support required features
    /// such as pushed authorization requests, client ID metadata, or authorization response parameters.
    #[error("error-oauth-auth-server-12 Authorization response parameters, pushed requests, client ID metadata must be supported")]
    RequiredServerFeaturesMustBeSupported,
}
