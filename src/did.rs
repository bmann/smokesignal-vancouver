pub mod model {

    use serde::Deserialize;
    use serde_json::Value;
    use std::collections::HashMap;

    #[derive(Clone, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct Service {
        pub id: String,

        pub r#type: String,

        pub service_endpoint: String,
    }

    #[derive(Clone, Deserialize, Debug)]
    #[serde(tag = "type", rename_all = "camelCase")]
    pub enum VerificationMethod {
        Multikey {
            id: String,
            controller: String,
            public_key_multibase: String,
        },

        #[serde(untagged)]
        Other {
            #[serde(flatten)]
            extra: HashMap<String, Value>,
        },
    }

    #[derive(Clone, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct Document {
        pub id: String,
        pub also_known_as: Vec<String>,
        pub service: Vec<Service>,
    }

    impl Document {
        pub fn pds_endpoint(&self) -> Option<&str> {
            self.service
                .iter()
                .find(|service| service.r#type == "AtprotoPersonalDataServer")
                .map(|service| service.service_endpoint.as_str())
        }

        pub fn primary_handle(&self) -> Option<&str> {
            self.also_known_as.first().map(|handle| {
                if let Some(trimmed) = handle.strip_prefix("at://") {
                    trimmed
                } else {
                    handle.as_str()
                }
            })
        }
    }

    #[cfg(test)]
    mod tests {
        use crate::did::model::Document;

        #[test]
        fn test_deserialize() {
            let document = serde_json::from_str::<Document>(
                r##"{"@context":["https://www.w3.org/ns/did/v1","https://w3id.org/security/multikey/v1","https://w3id.org/security/suites/secp256k1-2019/v1"],"id":"did:plc:cbkjy5n7bk3ax2wplmtjofq2","alsoKnownAs":["at://ngerakines.me","at://nick.gerakines.net","at://nick.thegem.city","https://github.com/ngerakines","https://ngerakines.me/","dns:ngerakines.me"],"verificationMethod":[{"id":"did:plc:cbkjy5n7bk3ax2wplmtjofq2#atproto","type":"Multikey","controller":"did:plc:cbkjy5n7bk3ax2wplmtjofq2","publicKeyMultibase":"zQ3shXvCK2RyPrSLYQjBEw5CExZkUhJH3n1K2Mb9sC7JbvRMF"}],"service":[{"id":"#atproto_pds","type":"AtprotoPersonalDataServer","serviceEndpoint":"https://pds.cauda.cloud"}]}"##,
            );
            assert!(document.is_ok());

            let document = document.unwrap();
            assert_eq!(document.id, "did:plc:cbkjy5n7bk3ax2wplmtjofq2");
        }

        #[test]
        fn test_deserialize_unsupported_verification_method() {
            let documents = vec![
                r##"{"@context":["https://www.w3.org/ns/did/v1","https://w3id.org/security/multikey/v1","https://w3id.org/security/suites/secp256k1-2019/v1"],"id":"did:plc:cbkjy5n7bk3ax2wplmtjofq2","alsoKnownAs":["at://ngerakines.me","at://nick.gerakines.net","at://nick.thegem.city","https://github.com/ngerakines","https://ngerakines.me/","dns:ngerakines.me"],"verificationMethod":[{"id":"did:plc:cbkjy5n7bk3ax2wplmtjofq2#atproto","type":"Ed25519VerificationKey2020","controller":"did:plc:cbkjy5n7bk3ax2wplmtjofq2","publicKeyMultibase":"zQ3shXvCK2RyPrSLYQjBEw5CExZkUhJH3n1K2Mb9sC7JbvRMF"}],"service":[{"id":"#atproto_pds","type":"AtprotoPersonalDataServer","serviceEndpoint":"https://pds.cauda.cloud"}]}"##,
                r##"{"@context":["https://www.w3.org/ns/did/v1","https://w3id.org/security/multikey/v1","https://w3id.org/security/suites/secp256k1-2019/v1"],"id":"did:plc:cbkjy5n7bk3ax2wplmtjofq2","alsoKnownAs":["at://ngerakines.me","at://nick.gerakines.net","at://nick.thegem.city","https://github.com/ngerakines","https://ngerakines.me/","dns:ngerakines.me"],"verificationMethod":[{"id": "did:example:123#_Qq0UL2Fq651Q0Fjd6TvnYE-faHiOpRlPVQcY_-tA4A","type": "JsonWebKey2020","controller": "did:example:123","publicKeyJwk": {"crv": "Ed25519","x": "VCpo2LMLhn6iWku8MKvSLg2ZAoC-nlOyPVQaO3FxVeQ","kty": "OKP","kid": "_Qq0UL2Fq651Q0Fjd6TvnYE-faHiOpRlPVQcY_-tA4A"}}],"service":[{"id":"#atproto_pds","type":"AtprotoPersonalDataServer","serviceEndpoint":"https://pds.cauda.cloud"}]}"##,
            ];
            for document in documents {
                let document = serde_json::from_str::<Document>(document);
                assert!(document.is_ok());

                let document = document.unwrap();
                assert_eq!(document.id, "did:plc:cbkjy5n7bk3ax2wplmtjofq2");
            }
        }
    }
}

pub mod plc {
    use anyhow::Result;
    use thiserror::Error;

    use super::model::Document;

    /// Error types that can occur when working with PLC DIDs
    #[derive(Debug, Error)]
    pub enum PLCDIDError {
        /// Occurs when the HTTP request to fetch the DID document fails
        #[error("error-did-plc-1 HTTP request failed: {url} {error}")]
        HttpRequestFailed {
            /// The URL that was requested
            url: String,
            /// The underlying HTTP error
            error: reqwest::Error,
        },

        /// Occurs when the DID document cannot be parsed from the HTTP response
        #[error("error-did-plc-2 Failed to parse DID document: {url} {error}")]
        DocumentParseFailed {
            /// The URL that was requested
            url: String,
            /// The underlying parse error
            error: reqwest::Error,
        },
    }

    pub async fn query(
        http_client: &reqwest::Client,
        plc_hostname: &str,
        did: &str,
    ) -> Result<Document> {
        let url = format!("https://{}/{}", plc_hostname, did);

        http_client
            .get(&url)
            .send()
            .await
            .map_err(|error| PLCDIDError::HttpRequestFailed {
                url: url.clone(),
                error,
            })?
            .json::<Document>()
            .await
            .map_err(|error| PLCDIDError::DocumentParseFailed { url, error })
            .map_err(Into::into)
    }
}

pub mod web {
    use anyhow::Result;
    use thiserror::Error;

    use super::model::Document;

    /// Error types that can occur when working with Web DIDs
    #[derive(Debug, Error)]
    pub enum WebDIDError {
        /// Occurs when the DID is missing the 'did:web:' prefix
        #[error("error-did-web-1 Invalid DID format: missing 'did:web:' prefix")]
        InvalidDIDPrefix,

        /// Occurs when the DID is missing a hostname component
        #[error("error-did-web-2 Invalid DID format: missing hostname component")]
        MissingHostname,

        /// Occurs when the HTTP request to fetch the DID document fails
        #[error("error-did-web-3 HTTP request failed: {url} {error}")]
        HttpRequestFailed {
            /// The URL that was requested
            url: String,
            /// The underlying HTTP error
            error: reqwest::Error,
        },

        /// Occurs when the DID document cannot be parsed from the HTTP response
        #[error("error-did-web-4 Failed to parse DID document: {url} {error}")]
        DocumentParseFailed {
            /// The URL that was requested
            url: String,
            /// The underlying parse error
            error: reqwest::Error,
        },
    }

    pub async fn query(http_client: &reqwest::Client, did: &str) -> Result<Document> {
        // Parse DID and extract hostname and path components
        let mut parts = did
            .strip_prefix("did:web:")
            .ok_or(WebDIDError::InvalidDIDPrefix)?
            .split(':')
            .collect::<Vec<&str>>();

        let hostname = parts.pop().ok_or(WebDIDError::MissingHostname)?;

        // Construct URL based on whether path components exist
        let url = if parts.is_empty() {
            format!("https://{}/.well-known/did.json", hostname)
        } else {
            format!("https://{}/{}/did.json", hostname, parts.join("/"))
        };

        // Fetch and parse document
        http_client
            .get(&url)
            .send()
            .await
            .map_err(|error| WebDIDError::HttpRequestFailed {
                url: url.clone(),
                error,
            })?
            .json::<Document>()
            .await
            .map_err(|error| WebDIDError::DocumentParseFailed { url, error })
            .map_err(Into::into)
    }

    pub async fn query_hostname(http_client: &reqwest::Client, hostname: &str) -> Result<Document> {
        let url = format!("https://{}/.well-known/did.json", hostname);

        tracing::debug!(?url, "query_hostname");

        http_client
            .get(&url)
            .send()
            .await
            .map_err(|error| WebDIDError::HttpRequestFailed {
                url: url.clone(),
                error,
            })?
            .json::<Document>()
            .await
            .map_err(|error| WebDIDError::DocumentParseFailed { url, error })
            .map_err(Into::into)
    }
}