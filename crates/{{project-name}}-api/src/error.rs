//! Maps `{{crate_name}}_types::Error` onto HTTP responses in the documented
//! envelope.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use {{crate_name}}_types::{Error, ErrorCode};

/// Wrapper that lets handlers return `Result<T, ApiError>` and get the correct
/// status and envelope for free.
///
/// Holds no request id of its own: [`render`] stamps that in from the request
/// extensions, so there is no second place it could be set and forgotten.
#[derive(Debug)]
pub struct ApiError {
    inner: Error,
}

impl From<Error> for ApiError {
    fn from(inner: Error) -> Self {
        Self { inner }
    }
}

impl IntoResponse for ApiError {
    /// Renders the envelope.
    ///
    /// The request id is not available here — a handler's return value cannot see
    /// request extensions — so the error is stashed in the response extensions and
    /// `render` finishes the job once the middleware has the id.
    fn into_response(self) -> Response {
        let status =
            StatusCode::from_u16(self.inner.status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        let mut response = (
            status,
            Json(self.inner.to_envelope(crate::request_id::UNKNOWN)),
        )
            .into_response();
        response
            .extensions_mut()
            .insert(DeferredError(std::sync::Arc::new(self.inner)));

        response
    }
}

/// An error awaiting its request id. Present only on responses built from
/// `ApiError`, which is how `render` knows which responses to rewrite.
#[derive(Clone)]
pub struct DeferredError(pub std::sync::Arc<Error>);

/// Rewrites error bodies with the real request id, and logs them.
///
/// Runs as middleware so no route can forget it — the alternative, an extractor
/// argument on every handler, silently degrades to `"unknown"` the first time
/// someone adds an endpoint without it.
pub async fn render(request: axum::extract::Request, next: axum::middleware::Next) -> Response {
    let request_id = request
        .extensions()
        .get::<crate::request_id::RequestId>()
        .map(|id| id.as_str().to_owned())
        .unwrap_or_else(|| crate::request_id::UNKNOWN.to_owned());

    let response = next.run(request).await;

    let Some(DeferredError(error)) = response.extensions().get::<DeferredError>().cloned() else {
        return response;
    };

    // Logged here rather than in `into_response` so the id is always attached.
    if error.code == ErrorCode::Internal {
        tracing::error!(
            request_id = %request_id,
            error = ?error,
            "request failed with an internal error"
        );
    } else {
        // The cause is included: for a rejected body it holds serde's detail,
        // which is the only way to debug a misbehaving client after the fact.
        tracing::debug!(
            request_id = %request_id,
            code = ?error.code,
            cause = ?error.source,
            "request rejected"
        );
    }

    let status = response.status();
    (status, Json(error.to_envelope(request_id))).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_each_code_to_its_documented_status() {
        for (code, expected) in [
            (ErrorCode::InvalidRequest, 400),
            (ErrorCode::Forbidden, 403),
            (ErrorCode::NotFound, 404),
            (ErrorCode::Conflict, 409),
            (ErrorCode::UnprocessableEntity, 422),
            (ErrorCode::RateLimited, 429),
            (ErrorCode::Internal, 500),
        ] {
            let response = ApiError::from(Error::new(code)).into_response();
            assert_eq!(
                response.status().as_u16(),
                expected,
                "wrong status for {code:?}"
            );
        }
    }
}
