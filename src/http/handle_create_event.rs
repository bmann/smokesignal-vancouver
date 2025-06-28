use std::collections::BTreeMap;
use std::collections::HashMap;

use anyhow::Result;
use axum::extract::Query;
use axum::extract::State;
use axum::response::IntoResponse;
use axum_extra::extract::Cached;
use axum_extra::extract::Form;
use axum_htmx::HxBoosted;
use axum_htmx::HxRequest;
use axum_template::RenderHtml;
use chrono::Utc;
use http::Method;
use http::StatusCode;
use minijinja::context as template_context;
use serde::Deserialize;

use crate::atproto::auth::SimpleOAuthSessionProvider;
use crate::atproto::client::CreateRecordRequest;
use crate::atproto::client::OAuthPdsClient;
use crate::atproto::lexicon::community::lexicon::calendar::event::Event;
use crate::atproto::lexicon::community::lexicon::calendar::event::EventLink;
use crate::atproto::lexicon::community::lexicon::calendar::event::EventLocation;
use crate::atproto::lexicon::community::lexicon::calendar::event::Mode;
use crate::atproto::lexicon::community::lexicon::calendar::event::Status;
use crate::atproto::lexicon::community::lexicon::calendar::event::NSID;
use crate::atproto::lexicon::community::lexicon::location::Address;
use crate::contextual_error;
use crate::http::context::WebContext;
use crate::http::errors::CommonError;
use crate::http::errors::CreateEventError;
use crate::http::errors::WebError;
use crate::http::event_form::BuildEventContentState;
use crate::http::event_form::BuildEventForm;
use crate::http::event_form::BuildLinkForm;
use crate::http::event_form::BuildStartsForm;
use crate::http::middleware_auth::Auth;
use crate::http::middleware_i18n::Language;
use crate::http::timezones::supported_timezones;
use crate::http::utils::url_from_aturi;
use crate::select_template;
use crate::storage::event::event_insert;

use super::cache_countries::cached_countries;
use super::event_form::BuildLocationForm;

pub async fn handle_create_event(
    method: Method,
    State(web_context): State<WebContext>,
    Language(language): Language,
    Cached(auth): Cached<Auth>,
    HxRequest(hx_request): HxRequest,
    HxBoosted(hx_boosted): HxBoosted,
    Form(mut build_event_form): Form<BuildEventForm>,
) -> Result<impl IntoResponse, WebError> {
    let current_handle = auth.require(&web_context.config.destination_key, "/event")?;

    let is_development = cfg!(debug_assertions);

    let default_context = template_context! {
        current_handle,
        language => language.to_string(),
        canonical_url => format!("https://{}/event", web_context.config.external_base),
        is_development,
        create_event => true,
        submit_url => format!("/event"),
    };
    // <a href="/{{ handle_slug }}/{{ event_rkey }}" class="button">Cancel</a>

    let render_template = select_template!("create_event", hx_boosted, hx_request, language);

    let error_template = select_template!(hx_boosted, hx_request, language);

    let (default_tz, timezones) = supported_timezones(auth.0.as_ref());

    if build_event_form.build_state.is_none() {
        build_event_form.build_state = Some(BuildEventContentState::default());
    }

    let mut starts_form = BuildStartsForm::from(build_event_form.clone());
    if starts_form.build_state.is_none() {
        starts_form.build_state = Some(BuildEventContentState::default());
    }

    if starts_form.tz.is_none() {
        starts_form.tz = Some(default_tz.to_string());
    }

    let mut location_form = BuildLocationForm::from(build_event_form.clone());
    if location_form.build_state.is_none() {
        location_form.build_state = Some(BuildEventContentState::default());
    }

    let mut link_form = BuildLinkForm::from(build_event_form.clone());
    if link_form.build_state.is_none() {
        link_form.build_state = Some(BuildEventContentState::default());
    }

    if method == Method::GET {
        #[cfg(debug_assertions)]
        {
            build_event_form.name = Some("My awesome event".to_string());
            build_event_form.description = Some("A really great event.".to_string());
        }

        // Set default values for required fields
        if build_event_form.status.is_none() {
            build_event_form.status = Some("scheduled".to_string());
        }

        if build_event_form.mode.is_none() {
            build_event_form.mode = Some("inperson".to_string());
        }

        build_event_form.build_state = Some(BuildEventContentState::Selecting);
        starts_form.build_state = Some(BuildEventContentState::Selected);

        // Set default start time to 6:00 PM, 6 hours from now
        let now = Utc::now(); // + chrono::Duration::hours(6);

        // Parse default timezone string to a Tz object
        let parsed_tz = default_tz
            .parse::<chrono_tz::Tz>()
            .unwrap_or(chrono_tz::UTC);

        // Get the date in the target timezone
        let local_date = now.with_timezone(&parsed_tz).date_naive();

        // Create a datetime at 6:00 PM on that date
        if let Some(naive_dt) = local_date.and_hms_opt(18, 0, 0) {
            // Convert to timezone-aware datetime
            let local_dt = naive_dt.and_local_timezone(parsed_tz).single().unwrap();
            let utc_dt = local_dt.with_timezone(&Utc);

            // Format the date and time as expected by the form
            starts_form.starts_date = Some(local_dt.format("%Y-%m-%d").to_string());
            starts_form.starts_time = Some(local_dt.format("%H:%M").to_string());
            starts_form.starts_at = Some(utc_dt.to_string());
            starts_form.starts_display = Some(local_dt.format("%A, %B %-d, %Y %r %Z").to_string());

            build_event_form.starts_at = starts_form.starts_at.clone();
        }

        return Ok(RenderHtml(
            &render_template,
            web_context.engine.clone(),
            template_context! { ..default_context, ..template_context! {
                build_event_form,
                starts_form,
                location_form,
                link_form,
                timezones,
            }},
        )
        .into_response());
    }

    match build_event_form.build_state {
        Some(BuildEventContentState::Reset) => {
            build_event_form.build_state = Some(BuildEventContentState::Selecting);
            build_event_form.name = None;
            build_event_form.name_error = None;
            build_event_form.description = None;
            build_event_form.description_error = None;
            build_event_form.status = Some("planned".to_string());
            build_event_form.status_error = None;
            build_event_form.starts_at = None;
            build_event_form.starts_at_error = None;
            build_event_form.ends_at = None;
            build_event_form.ends_at_error = None;
            build_event_form.mode = Some("inperson".to_string());
            build_event_form.mode_error = None;
        }
        Some(BuildEventContentState::Selected) => {
            let found_errors =
                build_event_form.validate(&web_context.i18n_context.locales, &language);
            if found_errors {
                build_event_form.build_state = Some(BuildEventContentState::Selecting);
            } else {
                build_event_form.build_state = Some(BuildEventContentState::Selected);
            }

            if !found_errors {
                // 1. Compose an event record

                let now = Utc::now();

                let starts_at = build_event_form
                    .starts_at
                    .as_ref()
                    .and_then(|v| v.parse::<chrono::DateTime<Utc>>().ok());
                let ends_at = build_event_form
                    .ends_at
                    .as_ref()
                    .and_then(|v| v.parse::<chrono::DateTime<Utc>>().ok());

                let mode = build_event_form
                    .mode
                    .as_ref()
                    .and_then(|v| match v.as_str() {
                        "inperson" => Some(Mode::InPerson),
                        "virtual" => Some(Mode::Virtual),
                        "hybrid" => Some(Mode::Hybrid),
                        _ => None,
                    });

                let status = build_event_form
                    .status
                    .as_ref()
                    .and_then(|v| match v.as_str() {
                        "planned" => Some(Status::Planned),
                        "scheduled" => Some(Status::Scheduled),
                        "cancelled" => Some(Status::Cancelled),
                        "postponed" => Some(Status::Postponed),
                        "rescheduled" => Some(Status::Rescheduled),
                        _ => None,
                    });

                // Ensure we have auth data for the API call
                let auth_data = auth.1.ok_or(CommonError::NotAuthorized)?;
                let client_auth: SimpleOAuthSessionProvider =
                    SimpleOAuthSessionProvider::try_from(auth_data)?;

                let client = OAuthPdsClient {
                    http_client: &web_context.http_client,
                    pds: &current_handle.pds,
                };

                let locations = match &build_event_form.location_country {
                    Some(country) => vec![EventLocation::Address(Address::Current {
                        country: country.clone(),
                        postal_code: build_event_form.location_postal_code.clone(),
                        region: build_event_form.location_region.clone(),
                        locality: build_event_form.location_locality.clone(),
                        street: build_event_form.location_street.clone(),
                        name: build_event_form.location_name.clone(),
                    })],
                    None => vec![],
                };

                // Process link if provided
                let links = match &build_event_form.link_value {
                    Some(uri) => vec![EventLink::Current {
                        uri: uri.clone(),
                        name: build_event_form.link_name.clone(),
                    }],
                    None => vec![],
                };

                let the_record = Event::Current {
                    name: build_event_form
                        .name
                        .clone()
                        .ok_or(CreateEventError::NameNotSet)?,
                    description: build_event_form
                        .description
                        .clone()
                        .ok_or(CreateEventError::DescriptionNotSet)?,
                    created_at: now,
                    starts_at,
                    ends_at,
                    mode,
                    status,
                    locations,
                    uris: links,
                    extra: HashMap::default(),
                };

                let event_record = CreateRecordRequest {
                    repo: current_handle.did.clone(),
                    collection: NSID.to_string(),
                    validate: false,
                    record_key: None,
                    record: the_record.clone(),
                    swap_commit: None,
                };

                let create_record_result = client.create_record(&client_auth, event_record).await;

                if let Err(err) = create_record_result {
                    return contextual_error!(
                        web_context,
                        language,
                        error_template,
                        default_context,
                        err
                    );
                }

                // create_record_result is guaranteed to be Ok since we checked for Err above
                let create_record_result = create_record_result?;

                let event_insert_result = event_insert(
                    &web_context.pool,
                    &create_record_result.uri,
                    &create_record_result.cid,
                    &current_handle.did,
                    NSID,
                    &the_record,
                )
                .await;

                if let Err(err) = event_insert_result {
                    return contextual_error!(
                        web_context,
                        language,
                        error_template,
                        default_context,
                        err
                    );
                }

                let event_url =
                    url_from_aturi(&web_context.config.external_base, &create_record_result.uri)?;

                return Ok(RenderHtml(
                    &render_template,
                    web_context.engine.clone(),
                    template_context! { ..default_context, ..template_context! {
                        build_event_form,
                        starts_form,
                        location_form,
                        link_form,
                        operation_completed => true,
                        event_url,
                    }},
                )
                .into_response());
            }
        }
        _ => {}
    }

    Ok(RenderHtml(
        &render_template,
        web_context.engine.clone(),
        template_context! { ..default_context, ..template_context! {
            build_event_form,
            starts_form,
            timezones,
            location_form,
            link_form,
        }},
    )
    .into_response())
}

pub async fn handle_starts_at_builder(
    method: Method,
    State(web_context): State<WebContext>,
    Language(language): Language,
    Cached(auth): Cached<Auth>,
    HxRequest(hx_request): HxRequest,
    Form(mut starts_form): Form<BuildStartsForm>,
) -> Result<impl IntoResponse, WebError> {
    if !hx_request {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    }

    if auth.require_flat().is_err() {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    }

    let (default_tz, timezones) = supported_timezones(auth.0.as_ref());

    let is_development = cfg!(debug_assertions);

    let render_template = format!(
        "create_event.{}.starts_form.html",
        language.to_string().to_lowercase()
    );

    if starts_form.build_state.is_none() {
        starts_form.build_state = Some(BuildEventContentState::default());
    }
    if starts_form.tz.is_none() {
        starts_form.tz = Some(default_tz.to_string());
    }

    if method == Method::GET {
        return Ok(RenderHtml(
            &render_template,
            web_context.engine.clone(),
            template_context! {
                starts_form,
                is_development,
                timezones,
            },
        )
        .into_response());
    }

    if starts_form
        .build_state
        .as_ref()
        .is_some_and(|value| value == &BuildEventContentState::Reset)
    {
        starts_form.tz = Some(default_tz.to_string());
        starts_form.tz_error = None;
        starts_form.starts_at = None;
        starts_form.starts_time = None;
        starts_form.starts_date = None;
        starts_form.ends_at = None;
        starts_form.ends_time = None;
        starts_form.ends_date = None;
        starts_form.include_ends = None;
    }

    if starts_form
        .build_state
        .as_ref()
        .is_some_and(|value| value == &BuildEventContentState::Selected)
    {
        let found_errors = starts_form.validate(&web_context.i18n_context.locales, &language);
        if found_errors {
            starts_form.build_state = Some(BuildEventContentState::Selecting);
        } else {
            starts_form.build_state = Some(BuildEventContentState::Selected);

            if starts_form.ends_display.is_none() {
                starts_form.ends_display = Some("--".to_string());
            }
        }
    }

    Ok(RenderHtml(
        &render_template,
        web_context.engine.clone(),
        template_context! {
            starts_form,
            is_development,
            timezones,
        },
    )
    .into_response())
}

pub async fn handle_location_at_builder(
    method: Method,
    State(web_context): State<WebContext>,
    Language(language): Language,
    Cached(auth): Cached<Auth>,
    HxRequest(hx_request): HxRequest,
    Form(mut location_form): Form<BuildLocationForm>,
) -> Result<impl IntoResponse, WebError> {
    if !hx_request {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    }

    if auth.require_flat().is_err() {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    }

    let is_development = cfg!(debug_assertions);

    let render_template = format!(
        "create_event.{}.location_form.html",
        language.to_string().to_lowercase()
    );

    if location_form.build_state.is_none() {
        location_form.build_state = Some(BuildEventContentState::default());
    }

    if method == Method::GET {
        return Ok(RenderHtml(
            &render_template,
            web_context.engine.clone(),
            template_context! {
                location_form,
                is_development
            },
        )
        .into_response());
    }

    if location_form
        .build_state
        .as_ref()
        .is_some_and(|value| value == &BuildEventContentState::Reset)
    {
        location_form.location_country = None;
        location_form.location_country_error = None;
        location_form.location_name = None;
        location_form.location_name_error = None;
    }

    if location_form
        .build_state
        .as_ref()
        .is_some_and(|value| value == &BuildEventContentState::Selected)
    {
        let found_errors = location_form.validate(&web_context.i18n_context.locales, &language);
        if found_errors {
            location_form.build_state = Some(BuildEventContentState::Selecting);
        } else {
            location_form.build_state = Some(BuildEventContentState::Selected);
        }
    }

    Ok(RenderHtml(
        &render_template,
        web_context.engine.clone(),
        template_context! {
            location_form,
            is_development,
        },
    )
    .into_response())
}

pub async fn handle_link_at_builder(
    method: Method,
    State(web_context): State<WebContext>,
    Language(language): Language,
    Cached(auth): Cached<Auth>,
    HxRequest(hx_request): HxRequest,
    Form(mut link_form): Form<BuildLinkForm>,
) -> Result<impl IntoResponse, WebError> {
    if !hx_request {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    }

    if auth.require_flat().is_err() {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    }

    let is_development = cfg!(debug_assertions);

    let render_template = format!(
        "create_event.{}.link_form.html",
        language.to_string().to_lowercase()
    );

    if link_form.build_state.is_none() {
        link_form.build_state = Some(BuildEventContentState::default());
    }

    if method == Method::GET {
        return Ok(RenderHtml(
            &render_template,
            web_context.engine.clone(),
            template_context! {
                link_form,
                is_development
            },
        )
        .into_response());
    }

    if link_form
        .build_state
        .as_ref()
        .is_some_and(|value| value == &BuildEventContentState::Reset)
    {
        link_form.link_name = None;
        link_form.link_name_error = None;
        link_form.link_value = None;
        link_form.link_value_error = None;
    }

    if link_form
        .build_state
        .as_ref()
        .is_some_and(|value| value == &BuildEventContentState::Selected)
    {
        let found_errors = link_form.validate(&web_context.i18n_context.locales, &language);
        if found_errors {
            link_form.build_state = Some(BuildEventContentState::Selecting);
        } else {
            link_form.build_state = Some(BuildEventContentState::Selected);
        }
    }

    Ok(RenderHtml(
        &render_template,
        web_context.engine.clone(),
        template_context! {
            link_form,
            is_development,
        },
    )
    .into_response())
}

#[derive(Deserialize, Debug, Clone)]
pub struct LocationDataListHint {
    pub location_country: Option<String>,
}

pub async fn handle_location_datalist(
    State(web_context): State<WebContext>,
    HxRequest(hx_request): HxRequest,
    Query(location_country_hint): Query<LocationDataListHint>,
) -> Result<impl IntoResponse, WebError> {
    if !hx_request {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    }

    let all_countries = cached_countries()?;

    let locations = if let Some(value) = location_country_hint.location_country {
        prefixed((**all_countries).clone(), &value)
            .iter()
            .take(30)
            .map(|(k, v)| (v.clone(), k.clone()))
            .collect::<Vec<(String, String)>>()
    } else {
        all_countries
            .iter()
            .take(30)
            .map(|(k, v)| (v.clone(), k.clone()))
            .collect::<Vec<(String, String)>>()
    };

    Ok(RenderHtml(
        "create_event.countries_datalist.html",
        web_context.engine.clone(),
        template_context! {
            locations,
        },
    )
    .into_response())
}

// Nick: The next two methods were adapted from https://www.thecodedmessage.com/posts/prefix-ranges/ which has no license. Thank you.

fn upper_bound_from_prefix(prefix: &str) -> Option<String> {
    for i in (0..prefix.len()).rev() {
        if let Some(last_char_str) = prefix.get(i..) {
            let rest_of_prefix = {
                debug_assert!(prefix.is_char_boundary(i));
                &prefix[0..i]
            };

            let last_char = last_char_str
                .chars()
                .next()
                .expect("last_char_str will contain at least one char");
            let Some(last_char_incr) = (last_char..=char::MAX).nth(1) else {
                // Last character is highest possible code point.
                // Go to second-to-last character instead.
                continue;
            };

            let new_string = format!("{rest_of_prefix}{last_char_incr}");

            return Some(new_string);
        }
    }

    None
}

pub fn prefixed(mut set: BTreeMap<String, String>, prefix: &str) -> BTreeMap<String, String> {
    let mut set = set.split_off(prefix);

    if let Some(not_in_prefix) = upper_bound_from_prefix(prefix) {
        set.split_off(&not_in_prefix);
    }

    set
}
