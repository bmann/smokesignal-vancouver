CREATE TABLE handles (
    did varchar(512) PRIMARY KEY,
    handle varchar(512) NOT NULL,
    pds varchar(512) NOT NULL,
    language varchar(12) NOT NULL DEFAULT 'en-us',
    tz varchar(48) NOT NULL DEFAULT 'America/New_York',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    active_at TIMESTAMP WITH TIME ZONE DEFAULT NULL
);
CREATE TABLE oauth_requests (
    oauth_state varchar(512) PRIMARY KEY,
    issuer varchar(512) NOT NULL,
    did varchar(512) NOT NULL,
    nonce varchar(512) NOT NULL,
    pkce_verifier varchar(512) NOT NULL,
    secret_jwk_id varchar(64) NOT NULL,
    dpop_jwk JSON NOT NULL,
    destination varchar(512) DEFAULT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW() + (10 || ' minutes')::interval
);
CREATE INDEX idx_oauth_requests_did ON oauth_requests(did);
CREATE TABLE oauth_sessions (
    session_group varchar(32) PRIMARY KEY,
    access_token varchar(1024) NOT NULL,
    did varchar(512) NOT NULL,
    issuer varchar(512) NOT NULL,
    refresh_token varchar(1024) NOT NULL,
    secret_jwk_id varchar(64) NOT NULL,
    dpop_jwk JSON NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    access_token_expires_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW() + (30 || ' minutes')::interval,
    not_after TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW() + (24 || ' hours')::interval
);
CREATE INDEX idx_oauth_sessions_did ON oauth_sessions(did);