//! Request-id plumbing.
//!
//! `tower-http` generates the id and sets `x-request-id`; this makes it
//! reachable from error rendering and from handlers. Without it every
//! envelope reports `"unknown"` and a client-reported id cannot be matched to
//! a log line.

use axum::{
    extract::{FromRequestParts, Request},
    http::{header::HeaderName, request::Parts, HeaderMap},
    middleware::Next,
    response::Response,
};

/// The header `tower_http::request_id` writes.
pub const X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

/// Stand-in when no id reached us. An absent id is a tracing gap, never a reason
/// to reject an otherwise valid request.
pub const UNKNOWN: &str = "unknown";

/// The current request's id.
#[derive(Debug, Clone)]
pub struct RequestId(pub String);

impl RequestId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Reads the id from headers, falling back to [`UNKNOWN`].
///
/// The single place the header is parsed, so the extractor and the middleware
/// below cannot drift apart on the header name or the fallback.
fn from_headers(headers: &HeaderMap) -> String {
    headers
        .get(X_REQUEST_ID)
        .and_then(|value| value.to_str().ok())
        .unwrap_or(UNKNOWN)
        .to_owned()
}

/// Lets a handler take `RequestId` as an argument, for correlating its own logs.
impl<S: Send + Sync> FromRequestParts<S> for RequestId {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Prefers what `propagate` already stored, so a handler and its error
        // response always report the same id.
        Ok(parts
            .extensions
            .get::<Self>()
            .cloned()
            .unwrap_or_else(|| Self(from_headers(&parts.headers))))
    }
}

/// Copies the incoming request id into the request extensions so error rendering
/// can read it.
///
/// Middleware rather than a per-handler extractor argument: a new route cannot
/// forget to opt in, so no endpoint can silently start emitting `"unknown"`.
pub async fn propagate(mut request: Request, next: Next) -> Response {
    let id = RequestId(from_headers(request.headers()));
    request.extensions_mut().insert(id);

    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_header_when_present() {
        let mut headers = HeaderMap::new();
        headers.insert(X_REQUEST_ID, "req_abc".parse().unwrap());

        assert_eq!(from_headers(&headers), "req_abc");
    }

    #[test]
    fn falls_back_when_the_header_is_absent_or_unreadable() {
        assert_eq!(from_headers(&HeaderMap::new()), UNKNOWN);

        let mut headers = HeaderMap::new();
        // Non-UTF-8 bytes are unreadable as a string.
        headers.insert(
            X_REQUEST_ID,
            axum::http::HeaderValue::from_bytes(&[0xff]).unwrap(),
        );
        assert_eq!(from_headers(&headers), UNKNOWN);
    }
}
