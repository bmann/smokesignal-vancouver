use anyhow::Result;
use errors::ResolveError;
use futures_util::future::join3;
use hickory_resolver::{
    config::{NameServerConfigGroup, ResolverConfig, ResolverOpts},
    TokioAsyncResolver,
};
use std::collections::HashSet;
use std::time::Duration;

use crate::config::DnsNameservers;
use crate::did::web::query_hostname;

pub enum InputType {
    Handle(String),
    Plc(String),
    Web(String),
}

pub async fn resolve_handle_dns(
    dns_resolver: &TokioAsyncResolver,
    lookup_dns: &str,
) -> Result<String, ResolveError> {
    let lookup = dns_resolver
        .txt_lookup(&format!("_atproto.{}", lookup_dns))
        .await
        .map_err(ResolveError::DNSResolutionFailed)?;

    let dids = lookup
        .iter()
        .filter_map(|record| {
            record
                .to_string()
                .strip_prefix("did=")
                .map(|did| did.to_string())
        })
        .collect::<HashSet<String>>();

    if dids.len() > 1 {
        return Err(ResolveError::MultipleDIDsFound);
    }

    dids.iter().next().cloned().ok_or(ResolveError::NoDIDsFound)
}

pub async fn resolve_handle_http(
    http_client: &reqwest::Client,
    handle: &str,
) -> Result<String, ResolveError> {
    let lookup_url = format!("https://{}/.well-known/atproto-did", handle);

    http_client
        .get(lookup_url.clone())
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(ResolveError::HTTPResolutionFailed)?
        .text()
        .await
        .map_err(ResolveError::HTTPResolutionFailed)
        .and_then(|body| {
            if body.starts_with("did:") {
                Ok(body.trim().to_string())
            } else {
                Err(ResolveError::InvalidHTTPResolutionResponse)
            }
        })
}

pub fn parse_input(input: &str) -> Result<InputType, ResolveError> {
    let trimmed = {
        if let Some(value) = input.trim().strip_prefix("at://") {
            value.trim()
        } else if let Some(value) = input.trim().strip_prefix('@') {
            value.trim()
        } else {
            input.trim()
        }
    };
    if trimmed.is_empty() {
        return Err(ResolveError::InvalidInput);
    }
    if trimmed.starts_with("did:web:") {
        Ok(InputType::Web(trimmed.to_string()))
    } else if trimmed.starts_with("did:plc:") {
        Ok(InputType::Plc(trimmed.to_string()))
    } else {
        Ok(InputType::Handle(trimmed.to_string()))
    }
}

pub async fn resolve_handle(
    http_client: &reqwest::Client,
    dns_resolver: &TokioAsyncResolver,
    handle: &str,
) -> Result<String, ResolveError> {
    let trimmed = {
        if let Some(value) = handle.trim().strip_prefix("at://") {
            value
        } else if let Some(value) = handle.trim().strip_prefix('@') {
            value
        } else {
            handle.trim()
        }
    };

    let (dns_lookup, http_lookup, did_web_lookup) = join3(
        resolve_handle_dns(dns_resolver, trimmed),
        resolve_handle_http(http_client, trimmed),
        query_hostname(http_client, trimmed),
    )
    .await;

    tracing::debug!(
        ?handle,
        ?dns_lookup,
        ?http_lookup,
        ?did_web_lookup,
        "raw query results"
    );

    let did_web_lookup_did = did_web_lookup
        .map(|document| document.id)
        .map_err(ResolveError::DIDWebResolutionFailed);

    let results = vec![dns_lookup, http_lookup, did_web_lookup_did]
        .into_iter()
        .filter_map(|result| result.ok())
        .collect::<Vec<String>>();
    if results.is_empty() {
        return Err(ResolveError::NoDIDsFound);
    }

    tracing::debug!(?handle, ?results, "query results");

    let first = results[0].clone();
    if results.iter().all(|result| result == &first) {
        return Ok(first);
    }
    Err(ResolveError::ConflictingDIDsFound)
}

pub async fn resolve_subject(
    http_client: &reqwest::Client,
    dns_resolver: &TokioAsyncResolver,
    subject: &str,
) -> Result<String, ResolveError> {
    match parse_input(subject)? {
        InputType::Handle(handle) => resolve_handle(http_client, dns_resolver, &handle).await,
        InputType::Plc(did) | InputType::Web(did) => Ok(did),
    }
}

/// Creates a new DNS resolver with configuration based on app config.
///
/// If custom nameservers are configured in app config, they will be used.
/// Otherwise, the system default resolver configuration will be used.
pub fn create_resolver(nameservers: DnsNameservers) -> TokioAsyncResolver {
    // Initialize the DNS resolver with custom nameservers if configured
    let nameservers = nameservers.as_ref();
    let resolver_config = if !nameservers.is_empty() {
        // Use custom nameservers
        tracing::info!("Using custom DNS nameservers: {:?}", nameservers);
        let nameserver_group = NameServerConfigGroup::from_ips_clear(nameservers, 53, true);
        ResolverConfig::from_parts(None, vec![], nameserver_group)
    } else {
        // Use system default
        tracing::info!("Using system default DNS nameservers");
        ResolverConfig::default()
    };

    // TokioAsyncResolver::tokio returns an AsyncResolver directly, not a Result
    TokioAsyncResolver::tokio(resolver_config, ResolverOpts::default())
}

pub mod errors {
    use thiserror::Error;

    #[derive(Debug, Error)]
    pub enum ResolveError {
        #[error("error-resolve-1 Multiple DIDs resolved for method")]
        MultipleDIDsFound,

        #[error("error-resolve-2 No DIDs resolved for method")]
        NoDIDsFound,

        #[error("error-resolve-3 No DIDs resolved for method")]
        ConflictingDIDsFound,

        #[error("error-resolve-4 DNS resolution failed: {0:?}")]
        DNSResolutionFailed(hickory_resolver::error::ResolveError),

        #[error("error-resolve-5 HTTP resolution failed: {0:?}")]
        HTTPResolutionFailed(reqwest::Error),

        #[error("error-resolve-6 HTTP resolution failed")]
        InvalidHTTPResolutionResponse,

        #[error("error-resolve-7 HTTP resolution failed: {0:?}")]
        DIDWebResolutionFailed(anyhow::Error),

        #[error("error-resolve-8 Invalid input")]
        InvalidInput,
    }
}
