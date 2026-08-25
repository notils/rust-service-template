//! Liveness and readiness probes.
//!
//! The two are deliberately separate. Liveness answers "is this process alive"
//! and must not touch the database — if it did, a brief database blip would make
//! the orchestrator kill and restart otherwise-healthy instances. Readiness
//! answers "can this instance serve traffic", which does require the database.

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Serialize;
use utoipa::ToSchema;

use crate::state::AppState;

#[derive(Serialize, ToSchema)]
pub struct Health {
    status: &'static str,
    version: &'static str,
}

#[derive(Serialize, ToSchema)]
pub struct Readiness {
    status: &'static str,
    database: &'static str,
}

/// `GET /health` — process liveness. No dependencies checked.
#[utoipa::path(
    get,
    path = "/health",
    tag = "Health",
    responses((status = 200, description = "The process is alive.", body = Health)),
)]
pub async fn health() -> Json<Health> {
    Json(Health { status: "ok", version: env!("CARGO_PKG_VERSION") })
}

/// `GET /health/ready` — readiness, including a database round-trip.
///
/// Returns `503` when the database is unreachable so a load balancer stops
/// sending traffic to an instance that cannot serve it.
#[utoipa::path(
    get,
    path = "/health/ready",
    tag = "Health",
    responses(
        (status = 200, description = "The database is reachable.", body = Readiness),
        (status = 503, description = "The database is unreachable.", body = Readiness),
    ),
)]
pub async fn readiness(State(state): State<AppState>) -> impl IntoResponse {
    match state.db().ping().await {
        Ok(()) => (StatusCode::OK, Json(Readiness { status: "ok", database: "up" })),
        Err(err) => {
            tracing::warn!(error = ?err, "readiness probe failed");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(Readiness { status: "degraded", database: "down" }),
            )
        }
    }
}
