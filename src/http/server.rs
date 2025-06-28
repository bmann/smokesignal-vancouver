use std::time::Duration;

use axum::{
    http::HeaderValue,
    routing::{get, post},
    Router,
};
use axum_htmx::AutoVaryLayer;
use http::{
    header::{ACCEPT, ACCEPT_LANGUAGE},
    Method,
};
use tower_http::trace::TraceLayer;
use tower_http::{classify::ServerErrorsFailureClass, timeout::TimeoutLayer};
use tower_http::{cors::CorsLayer, services::ServeDir};
use tracing::Span;

use crate::http::{
    context::WebContext,
    handle_admin_denylist::{
        handle_admin_denylist, handle_admin_denylist_add, handle_admin_denylist_remove,
    },
    handle_admin_event::handle_admin_event,
    handle_admin_events::handle_admin_events,
    handle_admin_handles::{handle_admin_handles, handle_admin_nuke_identity},
    handle_admin_import_event::handle_admin_import_event,
    handle_admin_import_rsvp::handle_admin_import_rsvp,
    handle_admin_index::handle_admin_index,
    handle_admin_rsvp::handle_admin_rsvp,
    handle_admin_rsvps::handle_admin_rsvps,
    handle_create_event::{
        handle_create_event, handle_link_at_builder, handle_location_at_builder,
        handle_location_datalist, handle_starts_at_builder,
    },
    handle_create_rsvp::handle_create_rsvp,
    handle_edit_event::handle_edit_event,
    handle_import::{handle_import, handle_import_submit},
    handle_index::handle_index,
    handle_migrate_event::handle_migrate_event,
    handle_migrate_rsvp::handle_migrate_rsvp,
    handle_oauth_callback::handle_oauth_callback,
    handle_oauth_jwks::handle_oauth_jwks,
    handle_oauth_login::handle_oauth_login,
    handle_oauth_logout::handle_logout,
    handle_oauth_metadata::handle_oauth_metadata,
    handle_policy::{
        handle_acknowledgement, handle_cookie_policy, handle_privacy_policy,
        handle_terms_of_service,
    },
    handle_profile::handle_profile_view,
    handle_set_language::handle_set_language,
    handle_settings::{handle_language_update, handle_settings, handle_timezone_update},
    handle_view_event::handle_view_event,
    handle_view_feed::handle_view_feed,
    handle_view_rsvp::handle_view_rsvp,
};

pub fn build_router(web_context: WebContext) -> Router {
    let serve_dir = ServeDir::new(web_context.config.http_static_path.clone());

    Router::new()
        .route("/", get(handle_index))
        .route("/privacy-policy", get(handle_privacy_policy))
        .route("/terms-of-service", get(handle_terms_of_service))
        .route("/cookie-policy", get(handle_cookie_policy))
        .route("/acknowledgement", get(handle_acknowledgement))
        .route("/admin", get(handle_admin_index))
        .route("/admin/handles", get(handle_admin_handles))
        .route(
            "/admin/handles/nuke/{did}",
            post(handle_admin_nuke_identity),
        )
        .route("/admin/denylist", get(handle_admin_denylist))
        .route("/admin/denylist/add", post(handle_admin_denylist_add))
        .route("/admin/denylist/remove", post(handle_admin_denylist_remove))
        .route("/admin/events", get(handle_admin_events))
        .route("/admin/events/import", post(handle_admin_import_event))
        .route("/admin/event", get(handle_admin_event))
        .route("/admin/rsvps", get(handle_admin_rsvps))
        .route("/admin/rsvp", get(handle_admin_rsvp))
        .route("/admin/rsvps/import", post(handle_admin_import_rsvp))
        .route("/oauth/client-metadata.json", get(handle_oauth_metadata))
        .route("/.well-known/jwks.json", get(handle_oauth_jwks))
        .route("/oauth/login", get(handle_oauth_login))
        .route("/oauth/login", post(handle_oauth_login))
        .route("/oauth/callback", get(handle_oauth_callback))
        .route("/logout", get(handle_logout))
        .route("/language", post(handle_set_language))
        .route("/settings", get(handle_settings))
        .route("/settings/timezone", post(handle_timezone_update))
        .route("/settings/language", post(handle_language_update))
        .route("/import", get(handle_import))
        .route("/import", post(handle_import_submit))
        .route("/event", get(handle_create_event))
        .route("/event", post(handle_create_event))
        .route("/rsvp", get(handle_create_rsvp))
        .route("/rsvp", post(handle_create_rsvp))
        .route("/rsvps", get(handle_view_rsvp))
        .route("/event/starts", get(handle_starts_at_builder))
        .route("/event/starts", post(handle_starts_at_builder))
        .route("/event/location", get(handle_location_at_builder))
        .route("/event/location", post(handle_location_at_builder))
        .route("/event/location/datalist", get(handle_location_datalist))
        .route("/event/links", get(handle_link_at_builder))
        .route("/event/links", post(handle_link_at_builder))
        .route("/{handle_slug}/{event_rkey}/edit", get(handle_edit_event))
        .route("/{handle_slug}/{event_rkey}/edit", post(handle_edit_event))
        .route(
            "/{handle_slug}/{event_rkey}/migrate",
            get(handle_migrate_event),
        )
        .route(
            "/{handle_slug}/{event_rkey}/migrate-rsvp",
            get(handle_migrate_rsvp),
        )
        .route("/feed/{handle_slug}/{feed_rkey}", get(handle_view_feed))
        .route("/rsvp/{handle_slug}/{rsvp_rkey}", get(handle_view_rsvp))
        .route("/{handle_slug}/{event_rkey}", get(handle_view_event))
        .route("/{handle_slug}", get(handle_profile_view))
        .nest_service("/static", serve_dir.clone())
        .fallback_service(serve_dir)
        .layer((
            TraceLayer::new_for_http().on_failure(
                |err: ServerErrorsFailureClass, _latency: Duration, _span: &Span| {
                    tracing::error!(error = ?err, "Unhandled error: {err}");
                },
            ),
            TimeoutLayer::new(Duration::from_secs(10)),
        ))
        .layer(
            CorsLayer::new()
                .allow_origin(
                    web_context
                        .config
                        .external_base
                        .parse::<HeaderValue>()
                        .unwrap(),
                )
                .allow_methods([Method::GET, Method::POST])
                .allow_headers([ACCEPT_LANGUAGE, ACCEPT]),
        )
        .layer(AutoVaryLayer)
        .with_state(web_context.clone())
}
