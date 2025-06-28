use anyhow::Result;
use fluent::{bundle::FluentBundle, FluentArgs, FluentResource};
use std::collections::HashMap;
use unic_langid::LanguageIdentifier;

use crate::i18n::errors::I18nError;

pub type Bundle = FluentBundle<FluentResource, intl_memoizer::concurrent::IntlLangMemoizer>;

pub struct Locales(pub HashMap<LanguageIdentifier, Bundle>);

impl Locales {
    pub fn new(locales: Vec<LanguageIdentifier>) -> Self {
        let mut store = HashMap::new();
        for locale in &locales {
            let bundle: FluentBundle<FluentResource, intl_memoizer::concurrent::IntlLangMemoizer> =
                FluentBundle::new_concurrent(vec![locale.clone()]);
            store.insert(locale.clone(), bundle);
        }
        Self(store)
    }

    pub fn add_bundle(
        &mut self,
        locale: LanguageIdentifier,
        content: String,
    ) -> Result<(), I18nError> {
        let bundle = self.0.get_mut(&locale).ok_or(I18nError::InvalidLanguage)?;

        let resource = FluentResource::try_new(content)
            .map_err(|(_, errors)| I18nError::LanguageResourceFailed(errors))?;

        bundle
            .add_resource(resource)
            .map_err(I18nError::BundleLoadFailed)?;

        Ok(())
    }

    pub fn format_error(&self, locale: &LanguageIdentifier, bare: &str, partial: &str) -> String {
        let bundle = self.0.get(locale);
        if bundle.is_none() {
            return partial.to_string();
        }

        let bundle = bundle.unwrap();

        let bundle_message = bundle.get_message(bare);

        if bundle_message.is_none() {
            return partial.to_string();
        }

        let bundle_message = bundle_message.unwrap();

        let mut errors = Vec::new();

        if bundle_message.value().is_none() {
            return partial.to_string();
        }
        let bundle_message_value = bundle_message.value().unwrap();

        let formatted_pattern = bundle.format_pattern(bundle_message_value, None, &mut errors);

        formatted_pattern.to_string()
    }

    pub fn format_message(
        &self,
        locale: &LanguageIdentifier,
        message: &str,
        args: FluentArgs,
    ) -> String {
        let bundle = self.0.get(locale);
        if bundle.is_none() {
            return message.to_string();
        }

        let bundle = bundle.unwrap();

        let bundle_message = bundle.get_message(message);

        if bundle_message.is_none() {
            return message.to_string();
        }

        let bundle_message = bundle_message.unwrap();

        let mut errors = Vec::new();

        if bundle_message.value().is_none() {
            return message.to_string();
        }
        let bundle_message_value = bundle_message.value().unwrap();

        let formatted_pattern =
            bundle.format_pattern(bundle_message_value, Some(&args), &mut errors);

        formatted_pattern.to_string()
    }
}

#[cfg(feature = "embed")]
pub mod embed {
    use rust_embed::Embed;
    use unic_langid::LanguageIdentifier;

    use crate::i18n::{errors::I18nError, Locales};

    #[derive(Embed)]
    #[folder = "i18n/"]
    struct I18nAssets;

    pub fn populate_locale(
        supported_locales: &Vec<LanguageIdentifier>,
        locales: &mut Locales,
    ) -> Result<(), I18nError> {
        let locale_files = vec!["errors"];

        for locale in supported_locales {
            for file in &locale_files {
                let source_file = format!("{}/{}.ftl", locale.to_string().to_lowercase(), file);
                let i18n_asset = I18nAssets::get(&source_file).expect("locale file not found");
                let content = std::str::from_utf8(i18n_asset.data.as_ref())
                    .expect("invalid utf-8 in locale file");
                locales.add_bundle(locale.clone(), content.to_string())?;
            }
        }
        Ok(())
    }
}

#[cfg(feature = "reload")]
pub mod reload {
    use std::path::PathBuf;
    use unic_langid::LanguageIdentifier;

    use crate::i18n::{errors::I18nError, Locales};

    pub fn populate_locale(
        supported_locales: &Vec<LanguageIdentifier>,
        locales: &mut Locales,
    ) -> Result<(), I18nError> {
        let locale_files = vec!["errors"];

        for locale in supported_locales {
            let locale_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("i18n")
                .join(locale.to_string().to_lowercase());
            for file in &locale_files {
                let source_file = locale_dir.join(format!("{}.ftl", file));
                tracing::info!("Loading locale file: {:?}", source_file);
                let i18n_asset = std::fs::read(source_file).expect("failed to read locale file");
                let content =
                    std::str::from_utf8(&i18n_asset).expect("invalid utf-8 in locale file");
                locales.add_bundle(locale.clone(), content.to_string())?;
            }
        }
        Ok(())
    }
}

pub mod errors {
    use thiserror::Error;

    #[derive(Debug, Error)]
    pub enum I18nError {
        #[error("error-i18n-1 Invalid language")]
        InvalidLanguage,

        #[error("error-i18n-2 Language resource failed")]
        LanguageResourceFailed(Vec<fluent_syntax::parser::ParserError>),

        #[error("error-i18n-3 Bundle load failed")]
        BundleLoadFailed(Vec<fluent::FluentError>),
    }
}
