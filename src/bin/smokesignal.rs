use anyhow::Result;
use chrono::Duration;
use smokesignal::{
    http::{
        context::{AppEngine, I18nContext, WebContext},
        server::build_router,
    },
    i18n::Locales,
    resolve::create_resolver,
    storage::cache::create_cache_pool,
    task_refresh_tokens::{RefreshTokensTask, RefreshTokensTaskConfig},
};
use sqlx::PgPool;
use std::{env, str::FromStr};
use tokio::net::TcpListener;
use tokio::signal;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use tracing_subscriber::prelude::*;
use unic_langid::LanguageIdentifier;

#[cfg(feature = "embed")]
use smokesignal::i18n::embed::populate_locale;

#[cfg(feature = "embed")]
use smokesignal::http::templates::embed_env;

#[cfg(feature = "reload")]
use smokesignal::i18n::reload::populate_locale;

#[cfg(feature = "reload")]
use smokesignal::http::templates::reload_env;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "smokesignal=debug,info".into()),
        ))
        .with(tracing_subscriber::fmt::layer().pretty())
        .init();

    let version = smokesignal::config::version()?;

    env::args().for_each(|arg| {
        if arg == "--version" {
            println!("{version}");
            std::process::exit(0);
        }
    });

    let config = smokesignal::config::Config::new()?;

    let mut client_builder = reqwest::Client::builder();
    for ca_certificate in config.certificate_bundles.as_ref() {
        tracing::info!("Loading CA certificate: {:?}", ca_certificate);
        let cert = std::fs::read(ca_certificate)?;
        let cert = reqwest::Certificate::from_pem(&cert)?;
        client_builder = client_builder.add_root_certificate(cert);
    }

    client_builder = client_builder.user_agent(config.user_agent.clone());
    let http_client = client_builder.build()?;

    let pool = PgPool::connect(&config.database_url).await?;
    sqlx::migrate!().run(&pool).await?;

    let cache_pool = create_cache_pool(&config.redis_url)?;

    let supported_languages = vec![LanguageIdentifier::from_str("en-us")?];
    tracing::info!("Supported languages: {:?}", supported_languages);

    let mut locales = Locales::new(supported_languages.clone());

    populate_locale(&supported_languages, &mut locales)?;

    #[cfg(feature = "embed")]
    let jinja = embed_env::build_env(config.external_base.clone(), config.version.clone());

    #[cfg(feature = "reload")]
    let jinja = reload_env::build_env(&config.external_base, &config.version);

    // Initialize the DNS resolver with configuration from the app config
    let dns_resolver = create_resolver(config.dns_nameservers.clone());

    let web_context = WebContext::new(
        pool.clone(),
        cache_pool.clone(),
        AppEngine::from(jinja),
        &http_client,
        config.clone(),
        I18nContext::new(supported_languages, locales),
        dns_resolver,
    );

    let app = build_router(web_context.clone());

    let tracker = TaskTracker::new();
    let token = CancellationToken::new();

    {
        let tracker = tracker.clone();
        let inner_token = token.clone();

        let ctrl_c = async {
            signal::ctrl_c()
                .await
                .expect("failed to install Ctrl+C handler");
        };

        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("failed to install signal handler")
                .recv()
                .await;
        };

        tokio::spawn(async move {
            tokio::select! {
                () = inner_token.cancelled() => { },
                _ = terminate => {},
                _ = ctrl_c => {},
            }

            tracker.close();
            inner_token.cancel();
        });
    }

    {
        let task_config = RefreshTokensTaskConfig {
            sleep_interval: Duration::seconds(10),
            worker_id: "dev".to_string(),
            external_url_base: config.external_base.clone(),
            signing_keys: config.signing_keys.clone(),
            oauth_active_keys: config.oauth_active_keys.clone(),
        };
        let task = RefreshTokensTask::new(
            task_config,
            http_client.clone(),
            pool.clone(),
            cache_pool.clone(),
            token.clone(),
        );

        let inner_token = token.clone();
        tracker.spawn(async move {
            if let Err(err) = task.run().await {
                tracing::error!("Database task failed: {}", err);
            }
            inner_token.cancel();
        });
    }

    {
        let inner_config = config.clone();
        let http_port = *inner_config.http_port.as_ref();
        let inner_token = token.clone();
        tracker.spawn(async move {
            let bind_address = format!("0.0.0.0:{http_port}");
            tracing::info!("bind_address {bind_address}");
            let listener = TcpListener::bind(&bind_address).await.unwrap();

            let shutdown_token = inner_token.clone();
            let result = axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    tokio::select! {
                        () = shutdown_token.cancelled() => { }
                    }
                    tracing::info!("axum graceful shutdown complete");
                })
                .await;
            if let Err(err) = result {
                tracing::error!("axum task failed: {}", err);
            }

            inner_token.cancel();
        });
    }

    tracker.wait().await;

    Ok(())
}
