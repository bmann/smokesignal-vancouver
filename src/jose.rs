use base64::{engine::general_purpose, Engine as _};
use jwt::{Claims, Header};
use p256::{
    ecdsa::{
        signature::{Signer, Verifier},
        Signature, SigningKey, VerifyingKey,
    },
    PublicKey, SecretKey,
};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::encoding::ToBase64;
use crate::jose_errors::JoseError;

/// Signs a JWT token with the provided secret key, header, and claims
///
/// Creates a JSON Web Token (JWT) by:
/// 1. Base64URL encoding the header and claims
/// 2. Signing the encoded header and claims with the secret key
/// 3. Returning the complete JWT (header.claims.signature)
pub fn mint_token(
    secret_key: &SecretKey,
    header: &Header,
    claims: &Claims,
) -> Result<String, JoseError> {
    // Encode header and claims to base64url
    let header = header
        .to_base64()
        .map_err(|_| JoseError::SigningKeyNotFound)?;
    let claims = claims
        .to_base64()
        .map_err(|_| JoseError::SigningKeyNotFound)?;
    let content = format!("{}.{}", header, claims);

    // Create signature
    let signing_key = SigningKey::from(secret_key.clone());
    let signature: Signature = signing_key
        .try_sign(content.as_bytes())
        .map_err(JoseError::SigningFailed)?;

    // Return complete JWT
    Ok(format!(
        "{}.{}",
        content,
        general_purpose::URL_SAFE_NO_PAD.encode(signature.to_bytes())
    ))
}

/// Verifies a JWT token's signature and validates its claims
///
/// Performs the following validations:
/// 1. Checks token format is valid (three parts separated by periods)
/// 2. Decodes header and claims from base64url format
/// 3. Verifies the token signature using the provided public key
/// 4. Validates token expiration (if provided in claims)
/// 5. Validates token not-before time (if provided in claims)
/// 6. Returns the decoded claims if all validation passes
pub fn verify_token(token: &str, public_key: &PublicKey) -> Result<Claims, JoseError> {
    // Split token into its parts
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(JoseError::InvalidTokenFormat);
    }

    let encoded_header = parts[0];
    let encoded_claims = parts[1];
    let encoded_signature = parts[2];

    // Decode header
    let header_bytes = general_purpose::URL_SAFE_NO_PAD
        .decode(encoded_header)
        .map_err(|_| JoseError::InvalidHeader)?;

    let header: Header =
        serde_json::from_slice(&header_bytes).map_err(|_| JoseError::InvalidHeader)?;

    // Verify algorithm matches what we expect
    // We only support ES256 for now
    if header.algorithm.as_deref() != Some("ES256") {
        return Err(JoseError::UnsupportedAlgorithm);
    }

    // Decode claims
    let claims_bytes = general_purpose::URL_SAFE_NO_PAD
        .decode(encoded_claims)
        .map_err(|_| JoseError::InvalidClaims)?;

    let claims: Claims =
        serde_json::from_slice(&claims_bytes).map_err(|_| JoseError::InvalidClaims)?;

    // Decode signature
    let signature_bytes = general_purpose::URL_SAFE_NO_PAD
        .decode(encoded_signature)
        .map_err(|_| JoseError::InvalidSignature)?;

    let signature =
        Signature::try_from(signature_bytes.as_slice()).map_err(|_| JoseError::InvalidSignature)?;

    // Verify signature
    let verifying_key = VerifyingKey::from(public_key);
    let content = format!("{}.{}", encoded_header, encoded_claims);

    verifying_key
        .verify(content.as_bytes(), &signature)
        .map_err(|_| JoseError::SignatureVerificationFailed)?;

    // Get current timestamp for validation
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| JoseError::SystemTimeError)?
        .as_secs();

    // Validate expiration time if present
    if let Some(exp) = claims.jose.expiration {
        if now >= exp {
            return Err(JoseError::TokenExpired);
        }
    }

    // Validate not-before time if present
    if let Some(nbf) = claims.jose.not_before {
        if now < nbf {
            return Err(JoseError::TokenNotYetValid);
        }
    }

    // Return validated claims
    Ok(claims)
}

pub mod jwk {
    use elliptic_curve::JwkEcKey;
    use p256::SecretKey;
    use rand::rngs::OsRng;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
    pub struct WrappedJsonWebKey {
        #[serde(skip_serializing_if = "Option::is_none", default)]
        pub kid: Option<String>,

        #[serde(skip_serializing_if = "Option::is_none", default)]
        pub alg: Option<String>,

        #[serde(flatten)]
        pub jwk: JwkEcKey,
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct WrappedJsonWebKeySet {
        pub keys: Vec<WrappedJsonWebKey>,
    }

    pub fn generate() -> WrappedJsonWebKey {
        let secret_key = SecretKey::random(&mut OsRng);

        let kid = ulid::Ulid::new().to_string();

        WrappedJsonWebKey {
            kid: Some(kid),
            alg: Some("ES256".to_string()),
            jwk: secret_key.to_jwk(),
        }
    }
}

pub mod jwt {

    use std::collections::BTreeMap;

    use elliptic_curve::JwkEcKey;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
    pub struct Header {
        #[serde(rename = "alg", skip_serializing_if = "Option::is_none")]
        pub algorithm: Option<String>,

        #[serde(rename = "kid", skip_serializing_if = "Option::is_none")]
        pub key_id: Option<String>,

        #[serde(rename = "typ", skip_serializing_if = "Option::is_none")]
        pub type_: Option<String>,

        #[serde(rename = "jwk", skip_serializing_if = "Option::is_none")]
        pub json_web_key: Option<JwkEcKey>,
    }

    #[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
    pub struct Claims {
        #[serde(flatten)]
        pub jose: JoseClaims,
        #[serde(flatten)]
        pub private: BTreeMap<String, serde_json::Value>,
    }

    impl Claims {
        pub fn new(jose: JoseClaims) -> Self {
            Claims {
                jose,
                private: BTreeMap::new(),
            }
        }
    }

    pub type SecondsSinceEpoch = u64;

    #[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
    pub struct JoseClaims {
        #[serde(rename = "iss", skip_serializing_if = "Option::is_none")]
        pub issuer: Option<String>,

        #[serde(rename = "sub", skip_serializing_if = "Option::is_none")]
        pub subject: Option<String>,

        #[serde(rename = "aud", skip_serializing_if = "Option::is_none")]
        pub audience: Option<String>,

        #[serde(rename = "exp", skip_serializing_if = "Option::is_none")]
        pub expiration: Option<SecondsSinceEpoch>,

        #[serde(rename = "nbf", skip_serializing_if = "Option::is_none")]
        pub not_before: Option<SecondsSinceEpoch>,

        #[serde(rename = "iat", skip_serializing_if = "Option::is_none")]
        pub issued_at: Option<SecondsSinceEpoch>,

        #[serde(rename = "jti", skip_serializing_if = "Option::is_none")]
        pub json_web_token_id: Option<String>,

        #[serde(rename = "htm", skip_serializing_if = "Option::is_none")]
        pub http_method: Option<String>,

        #[serde(rename = "htu", skip_serializing_if = "Option::is_none")]
        pub http_uri: Option<String>,

        #[serde(rename = "nonce", skip_serializing_if = "Option::is_none")]
        pub nonce: Option<String>,

        #[serde(rename = "ath", skip_serializing_if = "Option::is_none")]
        pub auth: Option<String>,
    }
}
