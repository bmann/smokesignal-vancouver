use anyhow::Result;
use axum::extract::Path;
use axum::response::IntoResponse;
use axum_extra::extract::Query;
use axum_htmx::{HxBoosted, HxRequest};
use axum_template::RenderHtml;
use chrono_tz::Tz;
use http::StatusCode;
use minijinja::context as template_context;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::{
    contextual_error,
    http::{
        context::UserRequestContext,
        errors::{CommonError, WebError},
        event_view::EventView,
        pagination::{Pagination, PaginationView},
        tab_selector::{TabLink, TabSelector},
        utils::build_url,
    },
    select_template,
    storage::{
        errors::StorageError,
        event::{event_list_did_recently_updated, model::EventWithRole},
        handle::{handle_for_did, handle_for_handle},
    },
};

use super::event_view::hydrate_event_organizers;

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub enum ProfileTab {
    RecentlyUpdated,
}

impl fmt::Display for ProfileTab {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ProfileTab::RecentlyUpdated => write!(f, "recentlyupdated"),
        }
    }
}

impl From<TabSelector> for ProfileTab {
    fn from(_: TabSelector) -> Self {
        ProfileTab::RecentlyUpdated
    }
}

pub async fn handle_profile_view(
    ctx: UserRequestContext,
    HxRequest(hx_request): HxRequest,
    HxBoosted(hx_boosted): HxBoosted,
    Path(handle_slug): Path<String>,
    pagination: Query<Pagination>,
    tab_selector: Query<TabSelector>,
) -> Result<impl IntoResponse, WebError> {
    let default_context = template_context! {
        language => ctx.language.to_string(),
        current_handle => ctx.current_handle,
    };

    let render_template = select_template!("profile", hx_boosted, hx_request, ctx.language);
    let error_template = select_template!(false, hx_request, ctx.language);

    if !handle_slug.starts_with("did:web:")
        && !handle_slug.starts_with("did:plc:")
        && !handle_slug.starts_with('@')
    {
        return contextual_error!(
            ctx.web_context,
            ctx.language,
            error_template,
            default_context,
            CommonError::InvalidHandleSlug,
            StatusCode::NOT_FOUND
        );
    }

    let profile = {
        if let Some(handle_slug) = handle_slug.strip_prefix('@') {
            handle_for_handle(&ctx.web_context.pool, handle_slug).await
        } else if handle_slug.starts_with("did:") {
            handle_for_did(&ctx.web_context.pool, &handle_slug).await
        } else {
            Err(StorageError::HandleNotFound)
        }
    };

    if let Err(err) = profile {
        return contextual_error!(
            ctx.web_context,
            ctx.language,
            error_template,
            default_context,
            err,
            StatusCode::NOT_FOUND
        );
    }

    let profile = profile.unwrap();

    let is_self = ctx
        .current_handle
        .clone()
        .is_some_and(|inner_current_entity| inner_current_entity.did == profile.did);

    let default_context = template_context! {
        current_handle => ctx.current_handle,
        language => ctx.language.to_string(),
        canonical_url => format!("https://{}/{}", ctx.web_context.config.external_base, profile.did),
        profile,
        is_self,
    };

    let _ = {
        if let Some(current_handle) = ctx.current_handle.clone() {
            current_handle.tz.parse::<Tz>().unwrap_or(Tz::UTC)
        } else {
            profile.tz.parse::<Tz>().unwrap_or(Tz::UTC)
        }
    };

    let (page, page_size) = pagination.clamped();
    let tab: ProfileTab = tab_selector.0.into();
    let tab_name = tab.to_string();

    let events = {
        let tab_events: Result<Vec<EventWithRole>> = match tab {
            ProfileTab::RecentlyUpdated => event_list_did_recently_updated(
                &ctx.web_context.pool,
                &profile.did,
                page,
                page_size,
            )
            .await
            .map_err(|err| err.into()),
        };
        match tab_events {
            Ok(values) => values,
            Err(err) => {
                return contextual_error!(
                    ctx.web_context,
                    ctx.language,
                    error_template,
                    default_context,
                    err,
                    StatusCode::NOT_FOUND
                );
            }
        }
    };

    let organizer_handlers = hydrate_event_organizers(&ctx.web_context.pool, &events).await?;

    let mut events = events
        .iter()
        .filter_map(|event_view| {
            let organizer_maybe = organizer_handlers.get(&event_view.event.did);
            EventView::try_from((
                ctx.current_handle.as_ref(),
                organizer_maybe,
                &event_view.event,
            ))
            .ok()
        })
        .collect::<Vec<EventView>>();

    if let Err(err) =
        super::event_view::hydrate_event_rsvp_counts(&ctx.web_context.pool, &mut events).await
    {
        tracing::warn!("Failed to hydrate event counts: {}", err);
    }

    let params: Vec<(&str, &str)> = vec![("tab", &tab_name)];

    let pagination_view = PaginationView::new(page_size, events.len() as i64, page, params);

    if events.len() > page_size as usize {
        events.truncate(page_size as usize);
    }

    let tab_links = vec![TabLink {
        name: "recentlyupdated".to_string(),
        label: "Recently Updated".to_string(),
        url: build_url(
            &ctx.web_context.config.external_base,
            &format!("/{}", handle_slug),
            vec![Some(("tab", "upcoming"))],
        ),
        active: tab == ProfileTab::RecentlyUpdated,
    }];

    Ok((
        StatusCode::OK,
        RenderHtml(
            &render_template,
            ctx.web_context.engine.clone(),
            template_context! { ..default_context, ..template_context! {
                tab => tab.to_string(),
                tabs => tab_links,
                events,
                pagination => pagination_view,
            }},
        ),
    )
        .into_response())
}
