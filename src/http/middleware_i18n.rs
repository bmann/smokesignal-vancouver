use anyhow::Result;
use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
    response::Response,
};
use axum_extra::extract::{cookie::CookieJar, Cached};
use std::{cmp::Ordering, str::FromStr};
use tracing::{debug, instrument, trace};
use unic_langid::LanguageIdentifier;

use crate::http::{context::WebContext, middleware_auth::Auth};
use crate::i18n::errors::I18nError;

pub const COOKIE_LANG: &str = "lang";

/// Represents a language from the Accept-Language header with its quality value
#[derive(Clone, Debug)]
struct AcceptedLanguage {
    value: String, // The language tag string
    quality: f32,  // The quality value (q parameter), from 0.0 to 1.0
}

impl Eq for AcceptedLanguage {}

impl PartialEq for AcceptedLanguage {
    fn eq(&self, other: &Self) -> bool {
        // Languages are equal if they have the same quality and tag
        self.quality == other.quality && self.value.eq(&other.value)
    }
}

impl PartialOrd for AcceptedLanguage {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AcceptedLanguage {
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare by quality (higher is better)
        self.quality
            .partial_cmp(&other.quality)
            .unwrap_or(Ordering::Equal)
    }
}

impl FromStr for AcceptedLanguage {
    type Err = I18nError;

    #[instrument(level = "trace", err, ret)]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        // Split into language tag and quality value
        let parts: Vec<&str> = s.split(';').collect();

        // Get language tag
        let value = parts.first().ok_or(I18nError::InvalidLanguage)?.trim();

        if value.is_empty() {
            return Err(I18nError::InvalidLanguage);
        }

        // Parse quality value if present (default to 1.0)
        let quality = if parts.len() > 1 {
            parts[1]
                .trim()
                .strip_prefix("q=")
                .and_then(|q| q.parse::<f32>().ok())
                .unwrap_or(1.0)
        } else {
            1.0
        };

        // Clamp quality to valid range
        let quality = quality.clamp(0.0, 1.0);

        Ok(AcceptedLanguage {
            value: value.to_string(),
            quality,
        })
    }
}

/// Wrapper around LanguageIdentifier for the current request's language
#[derive(Clone, Debug)]
pub struct Language(pub LanguageIdentifier);

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::ops::Deref for Language {
    type Target = LanguageIdentifier;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<LanguageIdentifier> for Language {
    fn from(id: LanguageIdentifier) -> Self {
        Language(id)
    }
}

impl<S> FromRequestParts<S> for Language
where
    WebContext: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, context: &S) -> Result<Self, Self::Rejection> {
        trace!("Extracting Language from request");
        let web_context = WebContext::from_ref(context);
        let auth: Auth = Cached::<Auth>::from_request_parts(parts, context).await?.0;

        // 1. Try to get language from user's profile settings
        if let Some(handle) = &auth.0 {
            if let Ok(auth_lang) = handle.language.parse::<LanguageIdentifier>() {
                debug!(language = %auth_lang, "Using language from user profile");
                return Ok(Self(auth_lang));
            }
        }

        // 2. Try to get language from cookies
        let cookie_jar = CookieJar::from_headers(&parts.headers);
        if let Some(lang_cookie) = cookie_jar.get(COOKIE_LANG) {
            trace!(cookie_value = %lang_cookie.value(), "Found language cookie");

            for value_part in lang_cookie.value().split(',') {
                if let Ok(value) = value_part.parse::<LanguageIdentifier>() {
                    for lang in &web_context.i18n_context.supported_languages {
                        if lang.matches(&value, true, false) {
                            debug!(language = %lang, "Using language from cookie");
                            return Ok(Self(lang.clone()));
                        }
                    }
                }
            }
        }

        // 3. Try to get language from Accept-Language header
        let accept_languages = match parts.headers.get("accept-language") {
            Some(header) => {
                if let Ok(header_str) = header.to_str() {
                    trace!(header = %header_str, "Processing Accept-Language header");

                    let mut langs = header_str
                        .split(',')
                        .filter_map(|lang| {
                            let parsed = lang.parse::<AcceptedLanguage>().ok();
                            if parsed.is_none() {
                                trace!(lang = %lang, "Failed to parse language from header");
                            }
                            parsed
                        })
                        .collect::<Vec<AcceptedLanguage>>();

                    langs.sort_by(|a, b| b.cmp(a)); // Sort in descending order by quality
                    langs
                } else {
                    Vec::new()
                }
            }
            None => Vec::new(),
        };

        for accept_language in accept_languages {
            if let Ok(value) = accept_language.value.parse::<LanguageIdentifier>() {
                for lang in &web_context.i18n_context.supported_languages {
                    if lang.matches(&value, true, false) {
                        debug!(language = %lang, quality = %accept_language.quality, "Using language from Accept-Language header");
                        return Ok(Self(lang.clone()));
                    }
                }
            }
        }

        // 4. Fall back to default language
        let default_lang = &web_context.i18n_context.supported_languages[0];
        debug!(language = %default_lang, "Using default language");
        Ok(Self(default_lang.clone()))
    }
}
