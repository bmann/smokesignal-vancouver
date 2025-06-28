use std::env;

use anyhow::Result;
use smokesignal::config::{default_env, optional_env, version, CertificateBundles, DnsNameservers};
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "trace".into()),
        ))
        .with(tracing_subscriber::fmt::layer().pretty())
        .init();

    let certificate_bundles: CertificateBundles = optional_env("CERTIFICATE_BUNDLES").try_into()?;
    let default_user_agent = format!("smokesignal ({}; +https://smokesignal.events/)", version()?);
    let user_agent = default_env("USER_AGENT", &default_user_agent);
    let dns_nameservers: DnsNameservers = optional_env("DNS_NAMESERVERS").try_into()?;

    let mut client_builder = reqwest::Client::builder();
    for ca_certificate in certificate_bundles.as_ref() {
        tracing::info!("Loading CA certificate: {:?}", ca_certificate);
        let cert = std::fs::read(ca_certificate)?;
        let cert = reqwest::Certificate::from_pem(&cert)?;
        client_builder = client_builder.add_root_certificate(cert);
    }

    client_builder = client_builder.user_agent(user_agent);
    let http_client = client_builder.build()?;

    // Initialize the DNS resolver with configuration from the app config
    let dns_resolver = smokesignal::resolve::create_resolver(dns_nameservers);

    for subject in env::args() {
        let resolved_did =
            smokesignal::resolve::resolve_subject(&http_client, &dns_resolver, &subject).await;
        tracing::info!(?resolved_did, ?subject, "resolved subject");
    }

    Ok(())
}
