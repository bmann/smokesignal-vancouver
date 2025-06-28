use anyhow::Result;
use axum::{extract::Path, response::IntoResponse};
use axum_extra::extract::Form;
use axum_htmx::{HxBoosted, HxRequest};
use axum_template::RenderHtml;
use chrono::Utc;
use http::{Method, StatusCode};
use minijinja::context as template_context;

use crate::{
    atproto::{
        auth::SimpleOAuthSessionProvider,
        client::{OAuthPdsClient, PutRecordRequest},
        lexicon::community::lexicon::calendar::event::{
            Event as LexiconCommunityEvent, EventLink, EventLocation, Mode, NamedUri, Status,
            NSID as LexiconCommunityEventNSID,
        },
        lexicon::community::lexicon::location::Address,
    },
    contextual_error,
    http::context::UserRequestContext,
    http::errors::EditEventError,
    http::errors::{CommonError, WebError},
    http::event_form::BuildLocationForm,
    http::event_form::{BuildEventContentState, BuildEventForm, BuildLinkForm, BuildStartsForm},
    http::location_edit_status::{check_location_edit_status, LocationEditStatus},
    http::timezones::supported_timezones,
    http::utils::url_from_aturi,
    resolve::{parse_input, InputType},
    select_template,
    storage::{
        event::{event_get, event_update_with_metadata},
        handle::{handle_for_did, handle_for_handle},
    },
};

pub async fn handle_edit_event(
    ctx: UserRequestContext,
    method: Method,
    HxBoosted(hx_boosted): HxBoosted,
    HxRequest(hx_request): HxRequest,
    Path((handle_slug, event_rkey)): Path<(String, String)>,
    Form(mut build_event_form): Form<BuildEventForm>,
) -> Result<impl IntoResponse, WebError> {
    let current_handle = ctx
        .auth
        .require(&ctx.web_context.config.destination_key, "/")?;

    let default_context = template_context! {
        current_handle,
        language => ctx.language.to_string(),
        canonical_url => format!("https://{}/{}/{}/edit", ctx.web_context.config.external_base, handle_slug, event_rkey),
        create_event => false,
        submit_url => format!("/{}/{}/edit", handle_slug, event_rkey),
        cancel_url => format!("/{}/{}", handle_slug, event_rkey),
    };

    let render_template = select_template!("edit_event", hx_boosted, hx_request, ctx.language);
    let error_template = select_template!(hx_boosted, hx_request, ctx.language);

    // Lookup the event
    let profile = match parse_input(&handle_slug) {
        Ok(InputType::Handle(handle)) => handle_for_handle(&ctx.web_context.pool, &handle)
            .await
            .map_err(WebError::from),
        Ok(InputType::Plc(did) | InputType::Web(did)) => {
            handle_for_did(&ctx.web_context.pool, &did)
                .await
                .map_err(WebError::from)
        }
        _ => Err(WebError::from(EditEventError::InvalidHandleSlug)),
    }?;

    let lookup_aturi = format!(
        "at://{}/{}/{}",
        profile.did, LexiconCommunityEventNSID, event_rkey
    );

    // Check if the user is authorized to edit this event (must be the creator)
    if profile.did != current_handle.did {
        return contextual_error!(
            ctx.web_context,
            ctx.language,
            error_template,
            default_context,
            EditEventError::NotAuthorized,
            StatusCode::FORBIDDEN
        );
    }

    let event = event_get(&ctx.web_context.pool, &lookup_aturi).await;
    if let Err(err) = event {
        return contextual_error!(
            ctx.web_context,
            ctx.language,
            error_template,
            default_context,
            err,
            StatusCode::OK
        );
    }

    let event = event.unwrap();

    // Check if this is a community calendar event (we only support editing those)
    if event.lexicon != LexiconCommunityEventNSID {
        return contextual_error!(
            ctx.web_context,
            ctx.language,
            error_template,
            default_context,
            EditEventError::UnsupportedEventType,
            StatusCode::BAD_REQUEST
        );
    }

    // Try to parse the event data
    let community_event =
        match serde_json::from_value::<LexiconCommunityEvent>(event.record.0.clone()) {
            Ok(event) => event,
            Err(_) => {
                return contextual_error!(
                    ctx.web_context,
                    ctx.language,
                    error_template,
                    default_context,
                    CommonError::InvalidEventFormat,
                    StatusCode::BAD_REQUEST
                );
            }
        };

    let (default_tz, timezones) = supported_timezones(ctx.current_handle.as_ref());

    let parsed_tz = default_tz
        .parse::<chrono_tz::Tz>()
        .unwrap_or(chrono_tz::UTC);

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

    let is_development = cfg!(debug_assertions);

    // Check if event locations can be edited
    let location_edit_status = match &community_event {
        LexiconCommunityEvent::Current { locations, .. } => check_location_edit_status(locations),
    };

    // Set flags for template rendering
    let locations_editable = location_edit_status.is_editable();
    let location_edit_reason = location_edit_status.edit_reason();

    // For GET requests, populate the form with existing event data
    if method == Method::GET {
        // Extract data from the parsed community event
        match &community_event {
            LexiconCommunityEvent::Current {
                name,
                description,
                status,
                mode,
                starts_at,
                ends_at,
                uris,
                ..
            } => {
                build_event_form.name = Some(name.clone());
                build_event_form.description = Some(description.clone());

                // If we have a single address location, populate the form fields with its data
                if let LocationEditStatus::Editable(Address::Current {
                    country,
                    postal_code,
                    region,
                    locality,
                    street,
                    name,
                }) = &location_edit_status
                {
                    build_event_form.location_country = Some(country.clone());
                    build_event_form.location_postal_code = postal_code.clone();
                    build_event_form.location_region = region.clone();
                    build_event_form.location_locality = locality.clone();
                    build_event_form.location_street = street.clone();
                    build_event_form.location_name = name.clone();

                    location_form.location_country = Some(country.clone());
                    location_form.location_postal_code = postal_code.clone();
                    location_form.location_region = region.clone();
                    location_form.location_locality = locality.clone();
                    location_form.location_street = street.clone();
                    location_form.location_name = name.clone();
                }

                // If we have URIs, populate the link form with the first one
                if !uris.is_empty() {
                    let EventLink::Current { uri, name } = &uris[0];
                    build_event_form.link_value = Some(uri.clone());
                    build_event_form.link_name = name.clone();

                    link_form.link_value = Some(uri.clone());
                    link_form.link_name = name.clone();
                }

                // Convert status enum to string
                if let Some(status_val) = status {
                    build_event_form.status = Some(
                        match status_val {
                            Status::Planned => "planned",
                            Status::Scheduled => "scheduled",
                            Status::Cancelled => "cancelled",
                            Status::Postponed => "postponed",
                            Status::Rescheduled => "rescheduled",
                        }
                        .to_string(),
                    );
                }

                // Convert mode enum to string
                if let Some(mode_val) = mode {
                    build_event_form.mode = Some(
                        match mode_val {
                            Mode::InPerson => "inperson",
                            Mode::Virtual => "virtual",
                            Mode::Hybrid => "hybrid",
                        }
                        .to_string(),
                    );
                }

                // Set date/time fields
                if let Some(start_time) = starts_at {
                    let local_dt = start_time.with_timezone(&parsed_tz);

                    starts_form.starts_date = Some(local_dt.format("%Y-%m-%d").to_string());
                    starts_form.starts_time = Some(local_dt.format("%H:%M").to_string());
                    starts_form.starts_at = Some(start_time.to_string());
                    starts_form.starts_display =
                        Some(local_dt.format("%A, %B %-d, %Y %r %Z").to_string());

                    build_event_form.starts_at = starts_form.starts_at.clone();
                } else {
                    starts_form.starts_display = Some("--".to_string());
                }

                if let Some(end_time) = ends_at {
                    let local_dt = end_time.with_timezone(&parsed_tz);

                    starts_form.include_ends = Some(true);
                    starts_form.ends_date = Some(local_dt.format("%Y-%m-%d").to_string());
                    starts_form.ends_time = Some(local_dt.format("%H:%M").to_string());
                    starts_form.ends_at = Some(end_time.to_string());
                    starts_form.ends_display =
                        Some(local_dt.format("%A, %B %-d, %Y %r %Z").to_string());

                    build_event_form.ends_at = starts_form.ends_at.clone();
                } else {
                    starts_form.ends_display = Some("--".to_string());
                }
            }
        }

        build_event_form.build_state = Some(BuildEventContentState::Selected);
        starts_form.build_state = Some(BuildEventContentState::Selected);
        location_form.build_state = Some(BuildEventContentState::Selected);
        link_form.build_state = Some(BuildEventContentState::Selected);

        // Extract location information for template display
        let location_display_info = match &community_event {
            LexiconCommunityEvent::Current { locations, .. } => {
                if locations.is_empty() {
                    None
                } else {
                    // Format locations for display
                    let mut formatted_locations = Vec::new();

                    for loc in locations {
                        match loc {
                            EventLocation::Address(Address::Current {
                                country,
                                postal_code,
                                region,
                                locality,
                                street,
                                name,
                            }) => {
                                let mut data = serde_json::Map::new();
                                data.insert(
                                    "type".to_string(),
                                    serde_json::Value::String("address".to_string()),
                                );
                                data.insert(
                                    "country".to_string(),
                                    serde_json::Value::String(country.clone()),
                                );

                                if let Some(n) = name {
                                    data.insert(
                                        "name".to_string(),
                                        serde_json::Value::String(n.clone()),
                                    );
                                }
                                if let Some(s) = street {
                                    data.insert(
                                        "street".to_string(),
                                        serde_json::Value::String(s.clone()),
                                    );
                                }
                                if let Some(l) = locality {
                                    data.insert(
                                        "locality".to_string(),
                                        serde_json::Value::String(l.clone()),
                                    );
                                }
                                if let Some(r) = region {
                                    data.insert(
                                        "region".to_string(),
                                        serde_json::Value::String(r.clone()),
                                    );
                                }
                                if let Some(pc) = postal_code {
                                    data.insert(
                                        "postal_code".to_string(),
                                        serde_json::Value::String(pc.clone()),
                                    );
                                }

                                formatted_locations.push(serde_json::Value::Object(data));
                            }
                            EventLocation::Uri(NamedUri::Current { uri, name }) => {
                                let mut data = serde_json::Map::new();
                                data.insert(
                                    "type".to_string(),
                                    serde_json::Value::String("uri".to_string()),
                                );
                                data.insert(
                                    "uri".to_string(),
                                    serde_json::Value::String(uri.clone()),
                                );

                                if let Some(n) = name {
                                    data.insert(
                                        "name".to_string(),
                                        serde_json::Value::String(n.clone()),
                                    );
                                }

                                formatted_locations.push(serde_json::Value::Object(data));
                            }
                            _ => {
                                let mut data = serde_json::Map::new();
                                data.insert(
                                    "type".to_string(),
                                    serde_json::Value::String("unknown".to_string()),
                                );
                                formatted_locations.push(serde_json::Value::Object(data));
                            }
                        }
                    }

                    Some(formatted_locations)
                }
            }
        };

        return Ok((
            StatusCode::OK,
            RenderHtml(
                &render_template,
                ctx.web_context.engine.clone(),
                template_context! { ..default_context, ..template_context! {
                    build_event_form,
                    starts_form,
                    location_form,
                    link_form,
                    event_rkey,
                    handle_slug,
                    timezones,
                    is_development,
                    locations_editable,
                    location_edit_reason,
                    location_display_info,
                }},
            ),
        )
            .into_response());
    }

    // Process form state changes just like in create_event
    match build_event_form.build_state {
        Some(BuildEventContentState::Reset) => {
            build_event_form.build_state = Some(BuildEventContentState::Selecting);
            build_event_form.name = None;
            build_event_form.name_error = None;
            build_event_form.description = None;
            build_event_form.description_error = None;
            build_event_form.status = None;
            build_event_form.status_error = None;
            build_event_form.starts_at = None;
            build_event_form.starts_at_error = None;
            build_event_form.ends_at = None;
            build_event_form.ends_at_error = None;
            build_event_form.mode = None;
            build_event_form.mode_error = None;

            // Regenerate starts_form from the updated build_event_form to ensure date/time fields are synced
            starts_form = BuildStartsForm::from(build_event_form.clone());
            starts_form.build_state = Some(BuildEventContentState::Selecting);

            location_form = BuildLocationForm::from(build_event_form.clone());
            location_form.build_state = Some(BuildEventContentState::Selecting);

            link_form = BuildLinkForm::from(build_event_form.clone());
            link_form.build_state = Some(BuildEventContentState::Selecting);
        }
        Some(BuildEventContentState::Selected) => {
            let found_errors =
                build_event_form.validate(&ctx.web_context.i18n_context.locales, &ctx.language);
            if found_errors {
                build_event_form.build_state = Some(BuildEventContentState::Selecting);
            } else {
                build_event_form.build_state = Some(BuildEventContentState::Selected);
            }

            // TODO: Consider adding the event CID and rkey to the form and
            // comparing it before submission. If the event CID is different
            // than what is contained in the form, then it could help prevent
            // race conditions where the event is double edited.

            // Preserving "extra" fields from the original record to ensure
            // we don't lose any additional metadata during edits

            if !found_errors {
                // Compose an updated event record

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

                let client_auth: SimpleOAuthSessionProvider =
                    SimpleOAuthSessionProvider::try_from(ctx.auth.1.unwrap())?;

                let client = OAuthPdsClient {
                    http_client: &ctx.web_context.http_client,
                    pds: &current_handle.pds,
                };

                // Extract existing locations and URIs from the original record
                let (locations, uris) = match &community_event {
                    LexiconCommunityEvent::Current {
                        locations, uris, ..
                    } => {
                        // Check if locations are editable
                        let location_edit_status = check_location_edit_status(locations);

                        // If locations aren't editable but the form has location data, return an error
                        if !location_edit_status.is_editable()
                            && (build_event_form.location_country.is_some()
                                || build_event_form.location_street.is_some()
                                || build_event_form.location_locality.is_some()
                                || build_event_form.location_region.is_some()
                                || build_event_form.location_postal_code.is_some()
                                || build_event_form.location_name.is_some())
                        {
                            // Return appropriate error based on edit status
                            // Note: NoLocations case removed since it's now handled as Editable
                            let error = match location_edit_status {
                                LocationEditStatus::MultipleLocations => {
                                    EditEventError::MultipleLocationsPresent
                                }
                                LocationEditStatus::UnsupportedLocationType => {
                                    EditEventError::UnsupportedLocationType
                                }
                                _ => unreachable!(),
                            };

                            return contextual_error!(
                                ctx.web_context,
                                ctx.language,
                                error_template,
                                default_context,
                                error,
                                StatusCode::BAD_REQUEST
                            );
                        }

                        // Handle locations
                        let updated_locations = if location_edit_status.is_editable()
                            && build_event_form.location_country.is_some()
                        {
                            // Create a new Address from form data
                            let address = Address::Current {
                                country: build_event_form.location_country.clone().unwrap(),
                                postal_code: build_event_form.location_postal_code.clone(),
                                region: build_event_form.location_region.clone(),
                                locality: build_event_form.location_locality.clone(),
                                street: build_event_form.location_street.clone(),
                                name: build_event_form.location_name.clone(),
                            };

                            vec![EventLocation::Address(address)]
                        } else {
                            // Preserve existing locations
                            locations.clone()
                        };

                        // Handle links
                        let updated_uris = if build_event_form.link_value.is_some() {
                            let uri = build_event_form.link_value.clone().unwrap();
                            let name = build_event_form.link_name.clone();
                            vec![EventLink::Current { uri, name }]
                        } else {
                            uris.clone()
                        };

                        (updated_locations, updated_uris)
                    }
                };

                // Extract existing extra fields from the original record
                let extra = match &community_event {
                    LexiconCommunityEvent::Current { extra, .. } => extra.clone(),
                };

                let updated_record = LexiconCommunityEvent::Current {
                    name: build_event_form
                        .name
                        .clone()
                        .ok_or(CommonError::FieldRequired)?,
                    description: build_event_form
                        .description
                        .clone()
                        .ok_or(CommonError::FieldRequired)?,
                    created_at: match &community_event {
                        LexiconCommunityEvent::Current { created_at, .. } => *created_at,
                    },
                    starts_at,
                    ends_at,
                    mode,
                    status,
                    locations,
                    uris,
                    extra, // Use the preserved extra fields
                };

                // Update the record in ATP
                let update_record_request = PutRecordRequest {
                    repo: current_handle.did.clone(),
                    collection: LexiconCommunityEventNSID.to_string(),
                    record_key: event_rkey.clone(),
                    record: updated_record.clone(),
                    validate: false,
                    swap_commit: None,
                    swap_record: Some(event.cid.clone()),
                };

                let update_record_result =
                    client.put_record(&client_auth, update_record_request).await;

                if let Err(err) = update_record_result {
                    return contextual_error!(
                        ctx.web_context,
                        ctx.language,
                        error_template,
                        default_context,
                        err,
                        StatusCode::OK
                    );
                }

                let update_record_result = update_record_result.unwrap();

                let name = match &updated_record {
                    LexiconCommunityEvent::Current { name, .. } => name,
                };

                // Update the local record
                let event_update_result = event_update_with_metadata(
                    &ctx.web_context.pool,
                    &lookup_aturi,
                    &update_record_result.cid,
                    &updated_record,
                    name,
                )
                .await;

                if let Err(err) = event_update_result {
                    return contextual_error!(
                        ctx.web_context,
                        ctx.language,
                        error_template,
                        default_context,
                        err,
                        StatusCode::OK
                    );
                }

                let event_url =
                    url_from_aturi(&ctx.web_context.config.external_base, &lookup_aturi)?;

                return Ok((
                    StatusCode::OK,
                    RenderHtml(
                        &render_template,
                        ctx.web_context.engine.clone(),
                        template_context! { ..default_context, ..template_context! {
                            build_event_form,
                            starts_form,
                            location_form,
                            link_form,
                            operation_completed => true,
                            event_url,
                            event_rkey,
                            handle_slug,
                            timezones,
                            is_development,
                            locations_editable,
                            location_edit_reason,
                        }},
                    ),
                )
                    .into_response());
            }
        }
        _ => {}
    }

    Ok((
        StatusCode::OK,
        RenderHtml(
            &render_template,
            ctx.web_context.engine.clone(),
            template_context! { ..default_context, ..template_context! {
                build_event_form,
                starts_form,
                location_form,
                link_form,
                event_rkey,
                handle_slug,
                timezones,
                is_development,
                locations_editable,
                location_edit_reason,
            }},
        ),
    )
        .into_response())
}
