use anyhow::Result;
use axum_extra::extract::cookie::Key;
use base64::{engine::general_purpose, Engine as _};
use ordermap::OrderMap;
use p256::SecretKey;
use rand::seq::SliceRandom;

use crate::config_errors::ConfigError;
use crate::encoding_errors::EncodingError;
use crate::jose::jwk::WrappedJsonWebKeySet;

#[derive(Clone)]
pub struct HttpPort(u16);

#[derive(Clone)]
pub struct HttpCookieKey(Key);

#[derive(Clone)]
pub struct CertificateBundles(Vec<String>);

#[derive(Clone)]
pub struct SigningKeys(OrderMap<String, SecretKey>);

#[derive(Clone)]
pub struct OAuthActiveKeys(Vec<String>);

#[derive(Clone)]
pub struct AdminDIDs(Vec<String>);

#[derive(Clone)]
pub struct DnsNameservers(Vec<std::net::IpAddr>);

#[derive(Clone)]
pub struct Config {
    pub version: String,
    pub http_port: HttpPort,
    pub http_cookie_key: HttpCookieKey,
    pub http_static_path: String,
    pub external_base: String,
    pub certificate_bundles: CertificateBundles,
    pub user_agent: String,
    pub database_url: String,
    pub plc_hostname: String,
    pub signing_keys: SigningKeys,
    pub oauth_active_keys: OAuthActiveKeys,
    pub destination_key: SecretKey,
    pub redis_url: String,
    pub admin_dids: AdminDIDs,
    pub dns_nameservers: DnsNameservers,
}

impl Config {
    pub fn new() -> Result<Self> {
        let http_port: HttpPort = default_env("HTTP_PORT", "8080").try_into()?;

        let http_cookie_key: HttpCookieKey =
            require_env("HTTP_COOKIE_KEY").and_then(|value| value.try_into())?;

        let http_static_path = default_env("HTTP_STATIC_PATH", "static");

        let external_base = require_env("EXTERNAL_BASE")?;

        let certificate_bundles: CertificateBundles =
            optional_env("CERTIFICATE_BUNDLES").try_into()?;

        let default_user_agent =
            format!("smokesignal ({}; +https://smokesignal.events/)", version()?);

        let user_agent = default_env("USER_AGENT", &default_user_agent);

        let plc_hostname = default_env("PLC_HOSTNAME", "plc.directory");

        let database_url = default_env("DATABASE_URL", "sqlite://development.db");

        let signing_keys: SigningKeys =
            require_env("SIGNING_KEYS").and_then(|value| value.try_into())?;

        let oauth_active_keys: OAuthActiveKeys =
            require_env("OAUTH_ACTIVE_KEYS").and_then(|value| value.try_into())?;

        let destination_key = require_env("DESTINATION_KEY").and_then(|value| {
            signing_keys
                .0
                .get(&value)
                .cloned()
                .ok_or(ConfigError::InvalidDestinationKey.into())
        })?;

        let redis_url = default_env("REDIS_URL", "redis://valkey:6379/0");

        let admin_dids: AdminDIDs = optional_env("ADMIN_DIDS").try_into()?;

        let dns_nameservers: DnsNameservers = optional_env("DNS_NAMESERVERS").try_into()?;

        Ok(Self {
            version: version()?,
            http_port,
            http_static_path,
            external_base,
            certificate_bundles,
            user_agent,
            plc_hostname,
            database_url,
            signing_keys,
            oauth_active_keys,
            http_cookie_key,
            destination_key,
            redis_url,
            admin_dids,
            dns_nameservers,
        })
    }

    pub fn select_oauth_signing_key(&self) -> Result<(String, SecretKey)> {
        let key_id = self
            .oauth_active_keys
            .as_ref()
            .choose(&mut rand::thread_rng())
            .ok_or(ConfigError::SigningKeyNotFound)?
            .clone();
        let signing_key = self
            .signing_keys
            .as_ref()
            .get(&key_id)
            .ok_or(ConfigError::SigningKeyNotFound)?
            .clone();

        Ok((key_id, signing_key))
    }

    /// Check if a DID is in the admin allow list
    pub fn is_admin(&self, did: &str) -> bool {
        self.admin_dids.as_ref().contains(&did.to_string())
    }
}

pub fn require_env(name: &str) -> Result<String> {
    std::env::var(name).map_err(|_| ConfigError::EnvVarRequired(name.to_string()).into())
}

pub fn optional_env(name: &str) -> String {
    std::env::var(name).unwrap_or("".to_string())
}

pub fn default_env(name: &str, default_value: &str) -> String {
    std::env::var(name).unwrap_or(default_value.to_string())
}

pub fn version() -> Result<String> {
    option_env!("GIT_HASH")
        .or(option_env!("CARGO_PKG_VERSION"))
        .map(|val| val.to_string())
        .ok_or(ConfigError::VersionNotSet.into())
}

impl TryFrom<String> for HttpPort {
    type Error = anyhow::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            Ok(Self(80))
        } else {
            value
                .parse::<u16>()
                .map(Self)
                .map_err(|err| ConfigError::PortParsingFailed(err).into())
        }
    }
}

impl AsRef<u16> for HttpPort {
    fn as_ref(&self) -> &u16 {
        &self.0
    }
}

impl TryFrom<String> for HttpCookieKey {
    type Error = anyhow::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let mut decoded_key: [u8; 66] = [0; 66];
        general_purpose::STANDARD_NO_PAD
            .decode_slice(value, &mut decoded_key)
            .map_err(|err| anyhow::Error::from(ConfigError::CookieKeyDecodeFailed(err)))?;
        Key::try_from(&decoded_key[..64])
            .map_err(|_| anyhow::Error::from(ConfigError::CookieKeyProcessFailed))
            .map(Self)
    }
}

impl AsRef<Key> for HttpCookieKey {
    fn as_ref(&self) -> &Key {
        &self.0
    }
}

impl TryFrom<String> for CertificateBundles {
    type Error = anyhow::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(Self(
            value
                .split(';')
                .filter_map(|s| {
                    if s.is_empty() {
                        None
                    } else {
                        Some(s.to_string())
                    }
                })
                .collect::<Vec<String>>(),
        ))
    }
}

impl AsRef<Vec<String>> for CertificateBundles {
    fn as_ref(&self) -> &Vec<String> {
        &self.0
    }
}

impl AsRef<OrderMap<String, SecretKey>> for SigningKeys {
    fn as_ref(&self) -> &OrderMap<String, SecretKey> {
        &self.0
    }
}

impl TryFrom<String> for SigningKeys {
    type Error = anyhow::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let content = {
            if value.starts_with("/") {
                // Verify file exists before reading
                if !std::path::Path::new(&value).exists() {
                    return Err(ConfigError::SigningKeysFileNotFound(value).into());
                }
                std::fs::read(&value).map_err(ConfigError::ReadSigningKeysFailed)?
            } else {
                general_purpose::STANDARD
                    .decode(&value)
                    .map_err(EncodingError::Base64DecodingFailed)?
            }
        };

        // Validate content is not empty
        if content.is_empty() {
            return Err(ConfigError::EmptySigningKeysFile.into());
        }

        // Parse JSON with proper error handling
        let jwks = serde_json::from_slice::<WrappedJsonWebKeySet>(&content)
            .map_err(ConfigError::ParseSigningKeysFailed)?;

        // Validate JWKS contains keys
        if jwks.keys.is_empty() {
            return Err(ConfigError::MissingKeysInJWKS.into());
        }

        // Track keys that failed validation for better error reporting
        let mut validation_errors = Vec::new();

        let signing_keys = jwks
            .keys
            .iter()
            .filter_map(|key| {
                // Validate key has required fields
                if key.kid.is_none() {
                    validation_errors.push("Missing key ID (kid)".to_string());
                    return None;
                }

                if let (Some(key_id), secret_key) = (key.kid.clone(), key.jwk.clone()) {
                    // Verify the key_id format (should be a valid ULID)
                    if ulid::Ulid::from_string(&key_id).is_err() {
                        validation_errors.push(format!("Invalid key ID format: {}", key_id));
                        return None;
                    }

                    // Validate the secret key
                    match p256::SecretKey::from_jwk(&secret_key) {
                        Ok(secret_key) => Some((key_id, secret_key)),
                        Err(err) => {
                            validation_errors.push(format!("Invalid key {}: {}", key_id, err));
                            None
                        }
                    }
                } else {
                    None
                }
            })
            .collect::<OrderMap<String, SecretKey>>();

        // Check if we have any valid keys
        if signing_keys.is_empty() {
            if !validation_errors.is_empty() {
                return Err(ConfigError::SigningKeysValidationFailed(validation_errors).into());
            }
            return Err(ConfigError::EmptySigningKeys.into());
        }

        Ok(Self(signing_keys))
    }
}

impl AsRef<Vec<String>> for OAuthActiveKeys {
    fn as_ref(&self) -> &Vec<String> {
        &self.0
    }
}

impl TryFrom<String> for OAuthActiveKeys {
    type Error = anyhow::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let values = value
            .split(';')
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        if values.is_empty() {
            return Err(ConfigError::EmptyOAuthActiveKeys.into());
        }
        Ok(Self(values))
    }
}

<<<<<<< HEAD
<<<<<<< HEAD
=======
impl AsRef<Vec<String>> for InvitationActiveKeys {
    fn as_ref(&self) -> &Vec<String> {
        &self.0
    }
}

impl TryFrom<String> for InvitationActiveKeys {
    type Error = anyhow::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let values = value
            .split(';')
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        if values.is_empty() {
            return Err(ConfigError::EmptyInvitationActiveKeys.into());
        }
        Ok(Self(values))
    }
}

>>>>>>> 3a59650 (Initial commit)
=======
>>>>>>> 61c52fe (Add VS Code configuration and improve developer documentation)
impl AsRef<Vec<String>> for AdminDIDs {
    fn as_ref(&self) -> &Vec<String> {
        &self.0
    }
}

impl TryFrom<String> for AdminDIDs {
    type Error = anyhow::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        // Allow empty value for no admins
        if value.is_empty() {
            return Ok(Self(Vec::new()));
        }

        let admin_dids = value
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<String>>();

        Ok(Self(admin_dids))
    }
}

impl AsRef<Vec<std::net::IpAddr>> for DnsNameservers {
    fn as_ref(&self) -> &Vec<std::net::IpAddr> {
        &self.0
    }
}

impl TryFrom<String> for DnsNameservers {
    type Error = anyhow::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        // Allow empty value for default DNS configuration
        if value.is_empty() {
            return Ok(Self(Vec::new()));
        }

        let nameservers = value
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| {
                s.parse::<std::net::IpAddr>()
                    .map_err(|e| ConfigError::NameserverParsingFailed(s.to_string(), e))
            })
            .collect::<Result<Vec<std::net::IpAddr>, ConfigError>>()?;

        Ok(Self(nameservers))
    }
}
<<<<<<< HEAD
<<<<<<< HEAD
=======

// Default implementation for testing
#[cfg(test)]
impl Default for Config {
    fn default() -> Self {
        // Create a random key for testing
        let cookie_key_data = [0u8; 64];
        let http_cookie_key = HttpCookieKey(Key::from(&cookie_key_data));

        // Create empty collections
        let signing_keys = SigningKeys(OrderMap::new());
        let oauth_active_keys = OAuthActiveKeys(Vec::new());
        let invitation_active_keys = InvitationActiveKeys(Vec::new());
        let certificate_bundles = CertificateBundles(Vec::new());

        // Create a default admin DID for testing
        let admin_dids = AdminDIDs(vec!["did:plc:testadmin".to_string()]);

        // Create empty DNS nameservers list for testing
        let dns_nameservers = DnsNameservers(Vec::new());

        Self {
            version: "test-version".to_string(),
            http_port: HttpPort(8080),
            http_cookie_key,
            external_base: "https://test.example".to_string(),
            certificate_bundles,
            user_agent: "smokesignal-test".to_string(),
            database_url: "sqlite://test.db".to_string(),
            plc_hostname: "plc.test".to_string(),
            signing_keys,
            oauth_active_keys,
            invitation_active_keys,
            // For testing, this needs to be a valid P-256 key
            // This would normally come from the signing keys, but for tests
            // we'll create a dummy one - note that it won't actually be used.
            destination_key: SecretKey::random(&mut rand::thread_rng()),
            redis_url: "redis://localhost:6379".to_string(),
            admin_dids,
            dns_nameservers,
        }
    }
}
