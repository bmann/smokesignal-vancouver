use std::fmt;

use anyhow::Result;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
};
use axum_extra::extract::Cached;
use axum_htmx::HxBoosted;
use axum_template::RenderHtml;

use minijinja::context as template_context;
use serde::{Deserialize, Serialize};

use crate::{
    contextual_error,
    http::{
        context::WebContext,
        errors::WebError,
        event_view::{hydrate_event_organizers, hydrate_event_rsvp_counts, EventView},
        middleware_auth::Auth,
        middleware_i18n::Language,
        pagination::{Pagination, PaginationView},
        tab_selector::TabSelector,
    },
    select_template,
    storage::event::event_list_recently_updated,
};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub enum HomeTab {
    RecentlyUpdated,
}

impl fmt::Display for HomeTab {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HomeTab::RecentlyUpdated => write!(f, "recentlyupdated"),
        }
    }
}

impl From<TabSelector> for HomeTab {
    fn from(_: TabSelector) -> Self {
        HomeTab::RecentlyUpdated
    }
}

pub async fn handle_index(
    State(web_context): State<WebContext>,
    HxBoosted(hx_boosted): HxBoosted,
    Language(language): Language,
    Cached(auth): Cached<Auth>,
    pagination: Query<Pagination>,
    tab_selector: Query<TabSelector>,
) -> Result<impl IntoResponse, WebError> {
    let render_template = select_template!("index", hx_boosted, false, language);
    let error_template = select_template!(false, false, language);

    let (page, page_size) = pagination.clamped();
    let tab: HomeTab = tab_selector.0.into();
    let tab_name = tab.to_string();

    let events = {
        let tab_events = match tab {
            HomeTab::RecentlyUpdated => {
                event_list_recently_updated(&web_context.pool, page, page_size).await
            }
        };
        match tab_events {
            Ok(values) => values,
            Err(err) => {
                return contextual_error!(
                    web_context,
                    language,
                    error_template,
                    template_context! {},
                    err
                );
            }
        }
    };

    let organizer_handlers = hydrate_event_organizers(&web_context.pool, &events).await?;

    let mut events = events
        .iter()
        .filter_map(|event_view| {
            let organizer_maybe = organizer_handlers.get(&event_view.event.did);
            let event_view =
                EventView::try_from((auth.0.as_ref(), organizer_maybe, &event_view.event));

            match event_view {
                Ok(event_view) => Some(event_view),
                Err(err) => {
                    tracing::warn!(err = ?err, "error converting event view");
                    None
                }
            }
        })
        .collect::<Vec<EventView>>();

    if let Err(err) = hydrate_event_rsvp_counts(&web_context.pool, &mut events).await {
        tracing::warn!("Failed to hydrate event counts: {}", err);
    }

    let params: Vec<(&str, &str)> = vec![("tab", &tab_name)];

    let pagination_view = PaginationView::new(page_size, events.len() as i64, page, params);

    if events.len() > page_size as usize {
        events.truncate(page_size as usize);
    }

    Ok((
        http::StatusCode::OK,
        RenderHtml(
            &render_template,
            web_context.engine.clone(),
            template_context! {
                current_handle => auth.0,
                language => language.to_string(),
                canonical_url => format!("https://{}/", web_context.config.external_base),
                tab => tab.to_string(),
                events,
                pagination => pagination_view,
            },
        ),
    )
        .into_response())
}
