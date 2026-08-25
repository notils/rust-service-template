//! Route table.
//!
//! Built through `utoipa_axum::OpenApiRouter` rather than a plain
//! `axum::Router`: each handler's `#[utoipa::path]` attribute is the only
//! place its shape is described, and this assembles those into the OpenAPI
//! document served at `/docs` (raw spec at `/api-docs/openapi.json`) — so the
//! doc can only ever describe what is actually routed here.
//!
//! ⚠️ `path` inside `#[utoipa::path]` is relative to its nesting, and it is
//! also the literal route axum registers — not just documentation metadata.
//! If you nest a route table under a prefix (e.g. `.nest("/v1", v1())`), a
//! handler inside it must use a path relative to that prefix, never repeat
//! the prefix itself, or you'll double it in the real route (and only
//! notice by testing the actual endpoint, not by reading the generated spec).

pub mod health;

use axum::Router;
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

use crate::state::AppState;

/// Builds the router, plus the interactive docs it publishes at `/docs`.
/// Middleware is applied in `crate::server`.
pub fn router(state: AppState) -> Router {
    let (router, mut openapi) = OpenApiRouter::new()
        .routes(utoipa_axum::routes!(health::health))
        .routes(utoipa_axum::routes!(health::readiness))
        // Add your own domain routes here, e.g.:
        //   .nest("/v1", v1())
        .with_state(state)
        .split_for_parts();

    openapi.info = utoipa::openapi::Info::new("{{project-name}}", env!("CARGO_PKG_VERSION"));

    router.merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", openapi))
}

// Sketch for your own versioned API surface:
//
// fn v1() -> OpenApiRouter<AppState> {
//     OpenApiRouter::new()
//         .routes(utoipa_axum::routes!(my_module::my_handler))
// }
