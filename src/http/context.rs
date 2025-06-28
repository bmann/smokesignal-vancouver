use axum::extract::FromRef;
use axum::{
    extract::FromRequestParts,
    http::request::Parts,
    response::{IntoResponse, Response},
};
use axum_extra::extract::Cached;
use axum_template::engine::Engine;
use cookie::Key;
use hickory_resolver::TokioAsyncResolver;
use minijinja::context as template_context;
use std::{ops::Deref, sync::Arc};
use unic_langid::LanguageIdentifier;

#[cfg(feature = "reload")]
use minijinja_autoreload::AutoReloader;

#[cfg(feature = "reload")]
pub type AppEngine = Engine<AutoReloader>;

#[cfg(feature = "embed")]
use minijinja::Environment;

use crate::{
    config::Config,
    http::middleware_auth::Auth,
    http::middleware_i18n::Language,
    i18n::Locales,
    storage::handle::model::Handle,
    storage::{CachePool, StoragePool},
};

#[cfg(feature = "embed")]
pub type AppEngine = Engine<Environment<'static>>;

pub struct I18nContext {
    pub supported_languages: Vec<LanguageIdentifier>,
    pub locales: Locales,
}

pub struct InnerWebContext {
    pub engine: AppEngine,
    pub http_client: reqwest::Client,
    pub pool: StoragePool,
    pub cache_pool: CachePool,
    pub config: Config,
    pub i18n_context: I18nContext,
    pub dns_resolver: hickory_resolver::TokioAsyncResolver,
}

#[derive(Clone, FromRef)]
pub struct WebContext(pub Arc<InnerWebContext>);

impl Deref for WebContext {
    type Target = InnerWebContext;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl WebContext {
    pub fn new(
        pool: StoragePool,
        cache_pool: CachePool,
        engine: AppEngine,
        http_client: &reqwest::Client,
        config: Config,
        i18n_context: I18nContext,
        dns_resolver: TokioAsyncResolver,
    ) -> Self {
        Self(Arc::new(InnerWebContext {
            pool,
            cache_pool,
            engine,
            http_client: http_client.clone(),
            config,
            i18n_context,
            dns_resolver,
        }))
    }
}

impl I18nContext {
    pub fn new(supported_languages: Vec<LanguageIdentifier>, locales: Locales) -> Self {
        Self {
            supported_languages,
            locales,
        }
    }
}

impl FromRef<WebContext> for Key {
    fn from_ref(context: &WebContext) -> Self {
        context.0.config.http_cookie_key.as_ref().clone()
    }
}

// New structs for reducing handler function arguments

/// A context struct specifically for admin handlers
pub struct AdminRequestContext {
    pub web_context: WebContext,
    pub language: Language,
    pub admin_handle: Handle,
    pub auth: Auth,
}

impl<S> FromRequestParts<S> for AdminRequestContext
where
    S: Send + Sync,
    WebContext: FromRef<S>,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, context: &S) -> Result<Self, Self::Rejection> {
        // Extract the needed components
        let web_context = WebContext::from_ref(context);
        let language = Language::from_request_parts(parts, context).await?;
        let cached_auth = Cached::<Auth>::from_request_parts(parts, context).await?;

        // Validate user is an admin
        let admin_handle = match cached_auth.0.require_admin(&web_context.config) {
            Ok(handle) => handle,
            Err(err) => return Err(err.into_response()),
        };

        Ok(Self {
            web_context,
            language,
            admin_handle,
            auth: cached_auth.0,
        })
    }
}

/// Helper function to create standard template context for admin views
pub fn admin_template_context(
    ctx: &AdminRequestContext,
    canonical_url: &str,
) -> minijinja::value::Value {
    template_context! {
        language => ctx.language.to_string(),
        current_handle => ctx.admin_handle.clone(),
        canonical_url => canonical_url,
    }
}

/// A context struct for regular authenticated user handlers
pub struct UserRequestContext {
    pub web_context: WebContext,
    pub language: Language,
    pub current_handle: Option<Handle>,
    pub auth: Auth,
}

impl<S> FromRequestParts<S> for UserRequestContext
where
    S: Send + Sync,
    WebContext: FromRef<S>,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, context: &S) -> Result<Self, Self::Rejection> {
        // Extract the needed components
        let web_context = WebContext::from_ref(context);
        let language = Language::from_request_parts(parts, context).await?;
        let cached_auth = Cached::<Auth>::from_request_parts(parts, context).await?;

        Ok(Self {
            web_context,
            language,
            current_handle: cached_auth.0 .0.clone(),
            auth: cached_auth.0,
        })
    }
}
