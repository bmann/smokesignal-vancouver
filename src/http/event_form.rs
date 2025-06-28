use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{errors::expand_error, i18n::Locales};

use super::cache_countries::cached_countries;

#[derive(Debug, Error)]
pub enum BuildEventError {
    #[error("error-event-builder-1 Invalid Name")]
    InvalidName,

    #[error("error-event-builder-2 Invalid Description")]
    InvalidDescription,

    #[error("error-event-builder-3 Invalid Time Zone")]
    InvalidTimeZone,

    #[error("error-event-builder-4 Invalid Status")]
    InvalidStatus,

    #[error("error-event-builder-5 Invalid Mode")]
    InvalidMode,

    #[error("error-event-builder-6 Invalid Start Date/Time Format")]
    InvalidStartDateTime,

    #[error("error-event-builder-7 Invalid End Date/Time Format")]
    InvalidEndDateTime,

    #[error("error-event-builder-8 End Date/Time Must Be After Start Date/Time")]
    EndBeforeStart,

    #[error("error-event-builder-9 Address Location Country Missing")]
    LocationCountryRequired,

    #[error("error-event-builder-10 Invalid Address Location Country: {0}")]
    LocationCountryInvalid(String),

    #[error("error-event-builder-11 Invalid Address Location Locality")]
    InvalidLocationAddressLocality,

    #[error("error-event-builder-12 Invalid Address Location Region")]
    InvalidLocationAddressRegion,

    #[error("error-event-builder-13 Invalid Address Location Street")]
    InvalidLocationAddressStreet,

    #[error("error-event-builder-14 Invalid Address Location Postal Code")]
    InvalidLocationAddressPostalCode,

    #[error("error-event-builder-15 Invalid Address Location Name")]
    InvalidLocationAddressName,

    #[error("error-event-builder-16 Invalid Link URL")]
    InvalidLinkValue,

    #[error("error-event-builder-17 Invalid Link Name")]
    InvalidLinkName,
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Clone)]
pub enum BuildEventContentState {
    #[default]
    Reset,
    Selecting,
    Selected,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BuildStartsForm {
    pub build_state: Option<BuildEventContentState>,

    pub tz: Option<String>,
    pub tz_error: Option<String>,

    pub starts_date: Option<String>,
    pub starts_date_error: Option<String>,

    pub starts_time: Option<String>,
    pub starts_time_error: Option<String>,

    pub starts_at: Option<String>,
    pub starts_at_error: Option<String>,

    pub include_ends: Option<bool>,

    pub ends_date: Option<String>,
    pub ends_date_error: Option<String>,

    pub ends_time: Option<String>,
    pub ends_time_error: Option<String>,

    pub ends_at: Option<String>,
    pub ends_at_error: Option<String>,

    pub starts_display: Option<String>,
    pub ends_display: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BuildLocationForm {
    pub build_state: Option<BuildEventContentState>,

    pub location_country: Option<String>,
    pub location_country_error: Option<String>,

    pub location_street: Option<String>,
    pub location_street_error: Option<String>,

    pub location_locality: Option<String>,
    pub location_locality_error: Option<String>,

    pub location_region: Option<String>,
    pub location_region_error: Option<String>,

    pub location_postal_code: Option<String>,
    pub location_postal_code_error: Option<String>,

    pub location_name: Option<String>,
    pub location_name_error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BuildLinkForm {
    pub build_state: Option<BuildEventContentState>,

    pub link_name: Option<String>,
    pub link_name_error: Option<String>,

    pub link_value: Option<String>,
    pub link_value_error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BuildEventForm {
    pub build_state: Option<BuildEventContentState>,

    pub name: Option<String>,
    pub name_error: Option<String>,

    pub description: Option<String>,
    pub description_error: Option<String>,

    pub status: Option<String>,
    pub status_error: Option<String>,

    pub starts_at: Option<String>,
    pub starts_at_error: Option<String>,

    pub ends_at: Option<String>,
    pub ends_at_error: Option<String>,

    pub mode: Option<String>,
    pub mode_error: Option<String>,

    pub location_country: Option<String>,
    pub location_country_error: Option<String>,

    pub location_street: Option<String>,
    pub location_street_error: Option<String>,

    pub location_locality: Option<String>,
    pub location_locality_error: Option<String>,

    pub location_region: Option<String>,
    pub location_region_error: Option<String>,

    pub location_postal_code: Option<String>,
    pub location_postal_code_error: Option<String>,

    pub location_name: Option<String>,
    pub location_name_error: Option<String>,

    pub link_name: Option<String>,
    pub link_name_error: Option<String>,

    pub link_value: Option<String>,
    pub link_value_error: Option<String>,
}

impl From<BuildEventForm> for BuildLocationForm {
    fn from(build_event_form: BuildEventForm) -> Self {
        BuildLocationForm {
            build_state: build_event_form.build_state,
            location_country: None,
            location_country_error: None,
            location_name: None,
            location_name_error: None,
            location_street: None,
            location_street_error: None,
            location_locality: None,
            location_locality_error: None,
            location_region: None,
            location_region_error: None,
            location_postal_code: None,
            location_postal_code_error: None,
        }
    }
}

impl From<BuildEventForm> for BuildStartsForm {
    fn from(build_event_form: BuildEventForm) -> Self {
        BuildStartsForm {
            build_state: build_event_form.build_state,
            tz: None,
            tz_error: None,
            starts_date: None,
            starts_date_error: None,
            starts_time: None,
            starts_time_error: None,
            starts_at: build_event_form.starts_at,
            starts_at_error: None,
            include_ends: None,
            ends_date: None,
            ends_date_error: None,
            ends_time: None,
            ends_time_error: None,
            ends_at: build_event_form.ends_at,
            ends_at_error: None,
            starts_display: None,
            ends_display: None,
        }
    }
}

impl From<BuildEventForm> for BuildLinkForm {
    fn from(build_event_form: BuildEventForm) -> Self {
        BuildLinkForm {
            build_state: build_event_form.build_state,
            link_name: None,
            link_name_error: None,
            link_value: None,
            link_value_error: None,
        }
    }
}

impl BuildLocationForm {
    pub fn validate(
        &mut self,
        locales: &Locales,
        language: &unic_langid::LanguageIdentifier,
    ) -> bool {
        if let Some(location_country_value) = self.location_country.as_ref() {
            let all_countries = match cached_countries() {
                Ok(value) => value,
                Err(err) => {
                    let (err_bare, err_partial) = expand_error(err);
                    let error_message = locales.format_error(language, &err_bare, &err_partial);
                    self.location_country_error = Some(error_message);
                    return true;
                }
            };

            if !all_countries.contains_key(location_country_value) {
                let (err_bare, err_partial) = expand_error(
                    BuildEventError::LocationCountryInvalid(location_country_value.clone()),
                );
                let error_message = locales.format_error(language, &err_bare, &err_partial);
                self.location_country_error = Some(error_message);
                return true;
            }
        } else {
            let (err_bare, err_partial) = expand_error(BuildEventError::LocationCountryRequired);
            let error_message = locales.format_error(language, &err_bare, &err_partial);
            self.location_country_error = Some(error_message);
            return true;
        }

        let mut found_errors = false;

        if let Some(user_value) = &self.location_locality {
            let trimmed_user_value = user_value.trim();
            if trimmed_user_value.is_empty() || trimmed_user_value.len() > 200 {
                let (err_bare, err_partial) =
                    expand_error(BuildEventError::InvalidLocationAddressLocality);
                let error_message = locales.format_error(language, &err_bare, &err_partial);
                self.location_locality_error = Some(error_message);
                found_errors = true;
            }

            if trimmed_user_value != user_value {
                let trimmed_string = trimmed_user_value.to_string();
                self.location_locality = Some(trimmed_string);
                found_errors = true;
            }
        }

        if let Some(user_value) = &self.location_region {
            let trimmed_user_value = user_value.trim();
            if trimmed_user_value.is_empty() || trimmed_user_value.len() > 200 {
                let (err_bare, err_partial) =
                    expand_error(BuildEventError::InvalidLocationAddressRegion);
                let error_message = locales.format_error(language, &err_bare, &err_partial);
                self.location_region_error = Some(error_message);
                found_errors = true;
            }

            if trimmed_user_value != user_value {
                let trimmed_string = trimmed_user_value.to_string();
                self.location_region = Some(trimmed_string);
                found_errors = true;
            }
        }

        if let Some(user_value) = &self.location_street {
            let trimmed_user_value = user_value.trim();
            if trimmed_user_value.is_empty() || trimmed_user_value.len() > 200 {
                let (err_bare, err_partial) =
                    expand_error(BuildEventError::InvalidLocationAddressStreet);
                let error_message = locales.format_error(language, &err_bare, &err_partial);
                self.location_street_error = Some(error_message);
                found_errors = true;
            }

            if trimmed_user_value != user_value {
                let trimmed_string = trimmed_user_value.to_string();
                self.location_street = Some(trimmed_string);
                found_errors = true;
            }
        }

        if let Some(user_value) = &self.location_postal_code {
            let trimmed_user_value = user_value.trim();
            if trimmed_user_value.is_empty() || trimmed_user_value.len() > 200 {
                let (err_bare, err_partial) =
                    expand_error(BuildEventError::InvalidLocationAddressPostalCode);
                let error_message = locales.format_error(language, &err_bare, &err_partial);
                self.location_postal_code_error = Some(error_message);
                found_errors = true;
            }

            if trimmed_user_value != user_value {
                let trimmed_string = trimmed_user_value.to_string();
                self.location_postal_code = Some(trimmed_string);
                found_errors = true;
            }
        }

        if let Some(user_value) = &self.location_name {
            let trimmed_user_value = user_value.trim();
            if trimmed_user_value.is_empty() || trimmed_user_value.len() > 200 {
                let (err_bare, err_partial) =
                    expand_error(BuildEventError::InvalidLocationAddressName);
                let error_message = locales.format_error(language, &err_bare, &err_partial);
                self.location_name_error = Some(error_message);
                found_errors = true;
            }

            if trimmed_user_value != user_value {
                let trimmed_string = trimmed_user_value.to_string();
                self.location_name = Some(trimmed_string);
                found_errors = true;
            }
        }

        found_errors
    }
}

impl BuildLinkForm {
    pub fn validate(
        &mut self,
        locales: &Locales,
        language: &unic_langid::LanguageIdentifier,
    ) -> bool {
        let mut found_errors = false;

        // Validate link URL (required)
        if let Some(link_value) = &self.link_value {
            let trimmed_value = link_value.trim();

            // Check if the URL is valid
            if trimmed_value.is_empty()
                || trimmed_value.len() > 500
                || (!trimmed_value.starts_with("http://") && !trimmed_value.starts_with("https://"))
            {
                let (err_bare, err_partial) = expand_error(BuildEventError::InvalidLinkValue);
                let error_message = locales.format_error(language, &err_bare, &err_partial);
                self.link_value_error = Some(error_message);
                found_errors = true;
            }

            // Replace original value with trimmed value if different
            if trimmed_value != link_value {
                let trimmed_string = trimmed_value.to_string();
                self.link_value = Some(trimmed_string);
                found_errors = true;
            }
        } else {
            let (err_bare, err_partial) = expand_error(BuildEventError::InvalidLinkValue);
            let error_message = locales.format_error(language, &err_bare, &err_partial);
            self.link_value_error = Some(error_message);
            found_errors = true;
        }

        // Validate link name (optional)
        if let Some(name_value) = &self.link_name {
            let trimmed_name = name_value.trim();

            // Only validate if not empty
            if !trimmed_name.is_empty() && trimmed_name.len() > 200 {
                let (err_bare, err_partial) = expand_error(BuildEventError::InvalidLinkName);
                let error_message = locales.format_error(language, &err_bare, &err_partial);
                self.link_name_error = Some(error_message);
                found_errors = true;
            }

            // Replace original value with trimmed value if different
            if trimmed_name != name_value {
                let trimmed_string = trimmed_name.to_string();
                self.link_name = Some(trimmed_string);
                found_errors = true;
            }
        }

        found_errors
    }
}

impl BuildStartsForm {
    pub fn validate(
        &mut self,
        locales: &Locales,
        language: &unic_langid::LanguageIdentifier,
    ) -> bool {
        if self.tz.is_none() {
            let (err_bare, err_partial) = expand_error(BuildEventError::InvalidTimeZone);
            let error_message = locales.format_error(language, &err_bare, &err_partial);
            self.tz_error = Some(error_message);
            return true;
        }

        let tz = self.tz.as_ref().unwrap().parse();

        if tz.is_err() {
            let (err_bare, err_partial) = expand_error(BuildEventError::InvalidTimeZone);
            let error_message = locales.format_error(language, &err_bare, &err_partial);
            self.tz_error = Some(error_message);
            return true;
        }

        let has_starts = self.starts_date.is_some() && self.starts_time.is_some();

        let tz: chrono_tz::Tz = tz.unwrap();

        let mut found_errors = false;

        let starts_at = if has_starts {
            let date_str = self.starts_date.clone().unwrap_or_default();
            let time_str = self.starts_time.clone().unwrap_or_default();

            match crate::http::timezones::combine_html_datetime(&date_str, &time_str, tz) {
                Ok(utc_dt) => {
                    self.starts_at = Some(utc_dt.to_string());
                    self.starts_display = Some(
                        utc_dt
                            .with_timezone(&tz)
                            .format("%A, %B %-d, %Y %r %Z")
                            .to_string(),
                    );
                    Some(utc_dt)
                }
                Err(_) => {
                    found_errors = true;
                    let (err_bare, err_partial) =
                        expand_error(BuildEventError::InvalidStartDateTime);
                    let error_message = locales.format_error(language, &err_bare, &err_partial);
                    self.starts_at_error = Some(error_message);
                    None
                }
            }
        } else {
            None
        };

        if self.include_ends.is_some_and(|v| v) {
            let has_ends = self.ends_date.is_some() && self.ends_time.is_some();
            if has_starts && !has_ends {
                let (err_bare, err_partial) = expand_error(BuildEventError::InvalidEndDateTime);
                let error_message = locales.format_error(language, &err_bare, &err_partial);
                self.ends_date_error = Some(error_message);
                found_errors = true;
            }
            let ends_at = if has_starts && has_ends {
                let date_str = self.ends_date.clone().unwrap_or_default();
                let time_str = self.ends_time.clone().unwrap_or_default();

                match crate::http::timezones::combine_html_datetime(&date_str, &time_str, tz) {
                    Ok(utc_dt) => {
                        self.ends_at = Some(utc_dt.to_string());
                        self.ends_display = Some(
                            utc_dt
                                .with_timezone(&tz)
                                .format("%A, %B %-d, %Y %r %Z")
                                .to_string(),
                        );
                        Some(utc_dt)
                    }
                    Err(_) => {
                        found_errors = true;
                        let (err_bare, err_partial) =
                            expand_error(BuildEventError::InvalidEndDateTime);
                        let error_message = locales.format_error(language, &err_bare, &err_partial);
                        self.ends_at_error = Some(error_message);
                        None
                    }
                }
            } else {
                None
            };

            if starts_at.is_some_and(|start| ends_at.is_some_and(|end| start > end)) {
                let (err_bare, err_partial) = expand_error(BuildEventError::EndBeforeStart);
                let error_message = locales.format_error(language, &err_bare, &err_partial);
                self.ends_at_error = Some(error_message);
                found_errors = true;
            }
        }

        found_errors
    }
}

impl BuildEventForm {
    pub fn validate(
        &mut self,
        locales: &Locales,
        language: &unic_langid::LanguageIdentifier,
    ) -> bool {
        let mut found_errors = false;

        // Validate name field
        if let Some(name_value) = &self.name {
            // Properly handle whitespace by trimming
            let trimmed_name = name_value.trim();

            // Check length requirements
            if trimmed_name.len() < 10 || trimmed_name.len() > 500 {
                let (err_bare, err_partial) = expand_error(BuildEventError::InvalidName);
                let error_message = locales.format_error(language, &err_bare, &err_partial);
                self.name_error = Some(error_message);
                found_errors = true;
            }

            // Replace original value with trimmed value if different
            if trimmed_name != name_value {
                // Create a new string to avoid borrowing issues
                let trimmed_string = trimmed_name.to_string();
                // Assign the new value to self.name
                self.name = Some(trimmed_string);
            }
        } else {
            let (err_bare, err_partial) = expand_error(BuildEventError::InvalidName);
            let error_message = locales.format_error(language, &err_bare, &err_partial);
            self.name_error = Some(error_message);
            found_errors = true;
        }

        // Validate description field
        if let Some(desc_value) = &self.description {
            // Properly handle whitespace by trimming
            let trimmed_desc = desc_value.trim();

            // Check character limits
            if trimmed_desc.len() < 10 || trimmed_desc.len() > 3000 {
                let (err_bare, err_partial) = expand_error(BuildEventError::InvalidDescription);
                let error_message = locales.format_error(language, &err_bare, &err_partial);
                self.description_error = Some(error_message);
                found_errors = true;
            }

            // Replace original value with trimmed value if different
            if trimmed_desc != desc_value {
                // Create a new string to avoid borrowing issues
                let trimmed_string = trimmed_desc.to_string();
                // Assign the new value to self.description
                self.description = Some(trimmed_string);
            }
        } else {
            let (err_bare, err_partial) = expand_error(BuildEventError::InvalidDescription);
            let error_message = locales.format_error(language, &err_bare, &err_partial);
            self.description_error = Some(error_message);
            found_errors = true;
        }

        // Validate status field
        if let Some(status) = &self.status {
            let valid_statuses = [
                "planned",
                "scheduled",
                "cancelled",
                "postponed",
                "rescheduled",
            ];
            if !valid_statuses.contains(&status.as_str()) {
                let (err_bare, err_partial) = expand_error(BuildEventError::InvalidStatus);
                let error_message = locales.format_error(language, &err_bare, &err_partial);
                self.status_error = Some(error_message);
                found_errors = true;
            }
        } else {
            // Default to planned if not provided
            self.status = Some("planned".to_string());
        }

        // Validate mode field
        if let Some(mode) = &self.mode {
            let valid_modes = ["inperson", "virtual", "hybrid"];
            if !valid_modes.contains(&mode.as_str()) {
                let (err_bare, err_partial) = expand_error(BuildEventError::InvalidMode);
                let error_message = locales.format_error(language, &err_bare, &err_partial);
                self.mode_error = Some(error_message);
                found_errors = true;
            }
        } else {
            // Default to inperson if not provided
            self.mode = Some("inperson".to_string());
        }

        found_errors
    }
}
