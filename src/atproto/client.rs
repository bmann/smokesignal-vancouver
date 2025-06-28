use std::time::Duration;

use anyhow::Result;
use reqwest_chain::ChainMiddleware;
use reqwest_middleware::ClientBuilder;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tracing::Instrument;

// Standard timeout for all HTTP client operations
const HTTP_CLIENT_TIMEOUT_SECS: u64 = 8;

use crate::atproto::auth::OAuthSessionProvider;
use crate::atproto::errors::ClientError;
use crate::atproto::lexicon::com::atproto::repo::StrongRef;
use crate::atproto::xrpc::SimpleError;
use crate::http::handle_oauth_login::pkce_challenge;
use crate::http::utils::URLBuilder;
use crate::jose::jwt::{Claims, Header, JoseClaims};
use crate::jose::mint_token;
use crate::oauth::dpop::DpopRetry;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(bound = "T: Serialize + DeserializeOwned")]
pub struct CreateRecordRequest<T: DeserializeOwned> {
    pub repo: String,
    pub collection: String,

    #[serde(skip_serializing_if = "Option::is_none", default, rename = "rkey")]
    pub record_key: Option<String>,

    pub validate: bool,

    pub record: T,

    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        rename = "swapCommit"
    )]
    pub swap_commit: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(bound = "T: Serialize + DeserializeOwned")]
pub struct PutRecordRequest<T: DeserializeOwned> {
    pub repo: String,
    pub collection: String,

    #[serde(rename = "rkey")]
    pub record_key: String,

    pub validate: bool,

    pub record: T,

    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        rename = "swapCommit"
    )]
    pub swap_commit: Option<String>,

    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        rename = "swapRecord"
    )]
    pub swap_record: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum CreateRecordResponse {
    StrongRef(StrongRef),
    Error(SimpleError),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum PutRecordResponse {
    StrongRef(StrongRef),
    Error(SimpleError),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ListRecordsParams {
    pub repo: String,
    pub collection: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reverse: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ListRecord<T> {
    pub uri: String,
    pub cid: String,
    pub value: T,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ListRecordsResponse<T> {
    pub cursor: Option<String>,
    pub records: Vec<ListRecord<T>>,
}

pub struct OAuthPdsClient<'a> {
    pub http_client: &'a reqwest::Client,
    pub pds: &'a str,
}

impl OAuthPdsClient<'_> {
    pub async fn create_record<T: DeserializeOwned + Serialize>(
        &self,
        oauth_session: &impl OAuthSessionProvider,
        record: CreateRecordRequest<T>,
    ) -> Result<StrongRef, anyhow::Error> {
        let mut url_builder = URLBuilder::new(self.pds);
        url_builder.path("/xrpc/com.atproto.repo.createRecord");
        let url = url_builder.build();

        let dpop_secret_key = oauth_session.dpop_secret();
        let dpop_public_key = dpop_secret_key.public_key();
        let oauth_issuer = oauth_session.oauth_issuer();
        let oauth_access_token = oauth_session.oauth_access_token();

        let now = chrono::Utc::now();

        let dpop_proof_header = Header {
            type_: Some("dpop+jwt".to_string()),
            algorithm: Some("ES256".to_string()),
            json_web_key: Some(dpop_public_key.to_jwk()),
            ..Default::default()
        };

        let dpop_proof_claim = Claims::new(JoseClaims {
            issuer: Some(oauth_issuer.clone()),
            issued_at: Some(now.timestamp() as u64),
            expiration: Some((now + chrono::Duration::seconds(30)).timestamp() as u64),
            json_web_token_id: Some(ulid::Ulid::new().to_string()),
            http_method: Some("POST".to_string()),
            http_uri: Some(url.clone()),
            auth: Some(pkce_challenge(&oauth_access_token)),

            ..Default::default()
        });
        let dpop_proof_token = mint_token(&dpop_secret_key, &dpop_proof_header, &dpop_proof_claim)?;

        let dpop_retry = DpopRetry::new(
            dpop_proof_header.clone(),
            dpop_proof_claim.clone(),
            dpop_secret_key.clone(),
        );

        let dpop_retry_client = ClientBuilder::new(self.http_client.clone())
            .with(ChainMiddleware::new(dpop_retry.clone()))
            .build();

        let http_response = dpop_retry_client
            .post(url)
            .header("Authorization", &format!("DPoP {}", oauth_access_token))
            .header("DPoP", dpop_proof_token.as_str())
            .json(&record)
            .timeout(Duration::from_secs(HTTP_CLIENT_TIMEOUT_SECS))
            .send()
            .instrument(tracing::info_span!("create_record"))
            .await?;

        tracing::info!(
            "create_record response status: {:?}",
            http_response.status()
        );

        let create_record_respoonse = http_response.json::<CreateRecordResponse>().await;

        match create_record_respoonse {
            Ok(CreateRecordResponse::StrongRef(strong_ref)) => Ok(strong_ref),
            Ok(CreateRecordResponse::Error(err)) => {
                Err(ClientError::ServerError(err.error_message()).into())
            }
            Err(err) => Err(ClientError::CreateRecordResponseFailure(err).into()),
        }
    }

    pub async fn put_record<T: DeserializeOwned + Serialize>(
        &self,
        oauth_session: &impl OAuthSessionProvider,
        record: PutRecordRequest<T>,
    ) -> Result<StrongRef, anyhow::Error> {
        let mut url_builder = URLBuilder::new(self.pds);
        url_builder.path("/xrpc/com.atproto.repo.putRecord");
        let url = url_builder.build();

        let dpop_secret_key = oauth_session.dpop_secret();
        let dpop_public_key = dpop_secret_key.public_key();
        let oauth_issuer = oauth_session.oauth_issuer();
        let oauth_access_token = oauth_session.oauth_access_token();

        let now = chrono::Utc::now();

        let dpop_proof_header = Header {
            type_: Some("dpop+jwt".to_string()),
            algorithm: Some("ES256".to_string()),
            json_web_key: Some(dpop_public_key.to_jwk()),
            ..Default::default()
        };

        let dpop_proof_claim = Claims::new(JoseClaims {
            issuer: Some(oauth_issuer.clone()),
            issued_at: Some(now.timestamp() as u64),
            expiration: Some((now + chrono::Duration::seconds(30)).timestamp() as u64),
            json_web_token_id: Some(ulid::Ulid::new().to_string()),
            http_method: Some("POST".to_string()),
            http_uri: Some(url.clone()),
            auth: Some(pkce_challenge(&oauth_access_token)),

            ..Default::default()
        });
        let dpop_proof_token = mint_token(&dpop_secret_key, &dpop_proof_header, &dpop_proof_claim)?;

        let dpop_retry = DpopRetry::new(
            dpop_proof_header.clone(),
            dpop_proof_claim.clone(),
            dpop_secret_key.clone(),
        );

        let dpop_retry_client = ClientBuilder::new(self.http_client.clone())
            .with(ChainMiddleware::new(dpop_retry.clone()))
            .build();

        let http_response = dpop_retry_client
            .post(url)
            .header("Authorization", &format!("DPoP {}", oauth_access_token))
            .header("DPoP", dpop_proof_token.as_str())
            .json(&record)
            .timeout(Duration::from_secs(HTTP_CLIENT_TIMEOUT_SECS))
            .send()
            .instrument(tracing::info_span!("put_record"))
            .await?;

        tracing::info!("put_record response status: {:?}", http_response.status());

        let put_record_respoonse = http_response.json::<PutRecordResponse>().await;

        match put_record_respoonse {
            Ok(PutRecordResponse::StrongRef(strong_ref)) => Ok(strong_ref),
            Ok(PutRecordResponse::Error(err)) => {
                Err(ClientError::ServerError(err.error_message()).into())
            }
            Err(err) => Err(ClientError::PutRecordResponseFailure(err).into()),
        }
    }

    pub async fn list_records<T: DeserializeOwned>(
        &self,
        oauth_session: &impl OAuthSessionProvider,
        params: &ListRecordsParams,
    ) -> Result<ListRecordsResponse<T>, anyhow::Error> {
        let mut url_builder = URLBuilder::new(self.pds);
        url_builder.path("/xrpc/com.atproto.repo.listRecords");

        // Add query parameters
        url_builder.param("repo", &params.repo);
        url_builder.param("collection", &params.collection);

        if let Some(limit) = params.limit {
            url_builder.param("limit", &limit.to_string());
        }

        if let Some(cursor) = &params.cursor {
            url_builder.param("cursor", cursor);
        }

        if let Some(reverse) = params.reverse {
            url_builder.param("reverse", &reverse.to_string());
        }

        let url = url_builder.build();

        let dpop_secret_key = oauth_session.dpop_secret();
        let dpop_public_key = dpop_secret_key.public_key();
        let oauth_issuer = oauth_session.oauth_issuer();
        let oauth_access_token = oauth_session.oauth_access_token();

        let now = chrono::Utc::now();

        let dpop_proof_header = Header {
            type_: Some("dpop+jwt".to_string()),
            algorithm: Some("ES256".to_string()),
            json_web_key: Some(dpop_public_key.to_jwk()),
            ..Default::default()
        };

        let dpop_proof_claim = Claims::new(JoseClaims {
            issuer: Some(oauth_issuer.clone()),
            issued_at: Some(now.timestamp() as u64),
            expiration: Some((now + chrono::Duration::seconds(30)).timestamp() as u64),
            json_web_token_id: Some(ulid::Ulid::new().to_string()),
            http_method: Some("GET".to_string()),
            http_uri: Some(url.clone()),
            auth: Some(pkce_challenge(&oauth_access_token)),

            ..Default::default()
        });
        let dpop_proof_token = mint_token(&dpop_secret_key, &dpop_proof_header, &dpop_proof_claim)?;

        let dpop_retry = DpopRetry::new(
            dpop_proof_header.clone(),
            dpop_proof_claim.clone(),
            dpop_secret_key.clone(),
        );

        let dpop_retry_client = ClientBuilder::new(self.http_client.clone())
            .with(ChainMiddleware::new(dpop_retry.clone()))
            .build();

        let http_response = dpop_retry_client
            .get(url)
            .header("Authorization", &format!("DPoP {}", oauth_access_token))
            .header("DPoP", dpop_proof_token.as_str())
            .timeout(Duration::from_secs(HTTP_CLIENT_TIMEOUT_SECS))
            .send()
            .instrument(tracing::span!(tracing::Level::INFO, "list_records"))
            .await?;

        let result = http_response.json::<ListRecordsResponse<T>>().await?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::atproto::lexicon::community::lexicon::calendar::event::Event;

    use super::*;
    use anyhow::Result;

    #[test]
    fn location_record() -> Result<()> {
        let test_json = r#"{"repo":"nick","collection":"stuff","validate":false,"record":{"$type":"community.lexicon.calendar.event","name":"My awesome event","description":"A really cool event.","createdAt":"2024-08-04T09:45:00.000Z"}}"#;

        {
            // Serialize bare
            assert_eq!(
                serde_json::to_string(&CreateRecordRequest {
                    repo: "nick".to_string(),
                    collection: "stuff".to_string(),
                    validate: false,
                    record_key: None,
                    record: Event::Current {
                        name: "My awesome event".to_string(),
                        description: "A really cool event.".to_string(),
                        created_at: "2024-08-04T09:45:00.000Z".parse().unwrap(),
                        starts_at: None,
                        ends_at: None,
                        mode: None,
                        status: None,
                        locations: vec![],
                        uris: vec![],
                        extra: HashMap::default(),
                    },
                    swap_commit: None,
                })
                .unwrap(),
                test_json
            );
        }

        Ok(())
    }
}
