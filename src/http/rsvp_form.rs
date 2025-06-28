use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    errors::expand_error,
    i18n::Locales,
    storage::{event::event_get_cid, StoragePool},
};

#[derive(Debug, Error)]
pub enum BuildRSVPError {
    #[error("error-rsvp-builder-1 Invalid Subject")]
    InvalidSubject,

    #[error("error-rsvp-builder-2 Invalid Status")]
    InvalidStatus,
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Clone)]
pub enum BuildRsvpContentState {
    #[default]
    Reset,
    Selecting,
    Selected,
    Review,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BuildRSVPForm {
    pub build_state: Option<BuildRsvpContentState>,

    pub subject_aturi: Option<String>,
    pub subject_aturi_error: Option<String>,

    pub subject_cid: Option<String>,
    pub subject_cid_error: Option<String>,

    pub status: Option<String>,
    pub status_error: Option<String>,
}

impl BuildRSVPForm {
    pub async fn hydrate(
        &mut self,
        database_pool: &StoragePool,
        locales: &Locales,
        language: &unic_langid::LanguageIdentifier,
    ) {
        // If we don't have an AT-URI, we can't do anything.
        let subject_aturi = match self.subject_aturi.as_ref() {
            Some(uri) => uri,
            None => return,
        };

        // If we already have a CID, we don't need to hydrate it.
        if self.subject_cid.is_some() {
            return;
        }

        let cid_result = event_get_cid(database_pool, subject_aturi).await;
        match cid_result {
            Ok(cid_option) => {
                self.subject_cid = cid_option;
            }
            Err(err) => {
                let (err_bare, err_partial) = expand_error(err);
                let error_message = locales.format_error(language, &err_bare, &err_partial);
                self.subject_cid_error = Some(error_message);
            }
        }
    }

    pub fn validate(
        &mut self,
        _locales: &Locales,
        _language: &unic_langid::LanguageIdentifier,
    ) -> bool {
        // TODO: Ensure subject_aturi is set.

        // TODO: Ensure subject_cid is set.

        // TODO: Ensure status is a valid value.

        false
    }
}
