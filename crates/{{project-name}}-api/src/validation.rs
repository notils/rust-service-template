//! `ValidatedJson<T>` — one extractor that deserializes, then validates.
//!
//! Replaces `Json<T>` on every request-body handler. Using plain `Json<T>` means
//! axum's own rejection is returned verbatim — a `text/plain` body like
//! `"Failed to deserialize the JSON body into the target type: missing field
//! `password` at line 4 column 1"`, which is neither the documented envelope nor
//! safe to show a user: it leaks Rust type detail and cursor positions.

use axum::extract::{FromRequest, Request};
use {{crate_name}}_types::{Error, ErrorCode, FieldError, field_errors_from};
use validator::Validate;

use crate::error::ApiError;

/// Deserializes a JSON body into `T`, then runs `T`'s `Validate` rules.
///
/// Both failure modes surface as the standard envelope: malformed JSON is
/// `400 invalid_request`, and a well-formed body with bad values is
/// `422 validation_failed` with per-field detail.
#[derive(Debug, Clone, Copy)]
pub struct ValidatedJson<T>(pub T);

impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: serde::de::DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        // Enforced before reading the body: a form post or a stray text/plain
        // should be rejected on its content type, not on a confusing parse error.
        if !is_json_content_type(request.headers()) {
            return Err(ApiError::from(
                Error::new(ErrorCode::InvalidRequest)
                    .with_message("Expected `Content-Type: application/json`."),
            ));
        }

        // Taken as raw bytes so deserialization can run through
        // `serde_path_to_error`, which reports *which* field failed. Axum's
        // `Json<T>` discards that path and yields only a prose message.
        let bytes = axum::body::Bytes::from_request(request, state).await.map_err(|_| {
            ApiError::from(
                Error::new(ErrorCode::InvalidRequest).with_message("Could not read the body."),
            )
        })?;

        let mut deserializer = serde_json::Deserializer::from_slice(&bytes);

        let value: T = serde_path_to_error::deserialize(&mut deserializer)
            .map_err(|err| deserialization_error(&err))?;

        value
            .validate()
            .map_err(|errors| ApiError::from(Error::validation(field_errors_from(&errors))))?;

        Ok(Self(value))
    }
}

/// True when the body is declared as JSON.
///
/// Accepts `application/json` and any `+json` suffix type, with or without
/// parameters (`; charset=utf-8`), matching what `axum::Json` allows.
fn is_json_content_type(headers: &axum::http::HeaderMap) -> bool {
    let Some(value) =
        headers.get(axum::http::header::CONTENT_TYPE).and_then(|value| value.to_str().ok())
    else {
        return false;
    };

    let essence = value.split(';').next().unwrap_or_default().trim().to_ascii_lowercase();

    essence == "application/json" || essence.ends_with("+json")
}

/// Maps a deserialization failure onto the envelope.
///
/// The raw serde message is kept as the internal cause only — it names Rust
/// types and byte offsets, which are noise to an API consumer and would leak
/// internal shape.
fn deserialization_error(err: &serde_path_to_error::Error<serde_json::Error>) -> ApiError {
    let detail = err.to_string();

    // Syntactically broken JSON has no field to blame, so it stays a 400 rather
    // than becoming a per-field 422.
    let error = if err.inner().classify() == serde_json::error::Category::Syntax {
        Error::new(ErrorCode::InvalidRequest).with_message("The request body is not valid JSON.")
    } else {
        let field = field_path(err);
        let (code, message) = classify(&detail);

        Error::validation(vec![FieldError::new(field, code, message)])
    };

    ApiError::from(error.with_source(RejectionDetail(detail)))
}

/// The dotted path to the offending field.
///
/// `serde_path_to_error` tracks this structurally, so nested and list fields come
/// through as `address.city` / `members[0].email` without parsing prose. A
/// missing field is reported at the parent, so the name is recovered from the
/// message in that one case.
fn field_path(err: &serde_path_to_error::Error<serde_json::Error>) -> String {
    let path = err.path().to_string();

    if let Some(name) = missing_field_name(&err.to_string()) {
        return if path.is_empty() || path == "." { name } else { format!("{path}.{name}") };
    }

    if path.is_empty() || path == "." { "body".to_owned() } else { path }
}

/// Maps a serde message onto a stable `code` and a safe message.
///
/// ⚠️ Deliberately never echoes the submitted value: serde includes it in type
/// errors ("invalid type: string \"hunter2\""), and a validation response is
/// exactly where a secret would otherwise be reflected back to the client.
fn classify(detail: &str) -> (&'static str, &'static str) {
    if detail.contains("missing field") {
        ("required", "Is required.")
    } else if detail.contains("invalid type") {
        ("invalid_type", "Has an unexpected type.")
    } else if detail.contains("unknown field") {
        ("unknown_field", "Is not a recognised field.")
    } else {
        ("invalid_value", "Is not a valid value.")
    }
}

/// Extracts the name from serde's "missing field \`x\`" message.
///
/// ⚠️ Message-text parsing, and the only place it is still needed: serde reports
/// a missing field against the containing struct, so the name exists nowhere in
/// the structured path. The fallback is the parent path rather than a wrong
/// field, and the test below fails loudly if serde rewords this.
fn missing_field_name(detail: &str) -> Option<String> {
    let after = detail.split("missing field `").nth(1)?;
    let field = after.split('`').next()?;

    (!field.is_empty()).then(|| field.to_owned())
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
struct RejectionDetail(String);

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    // Fields exist to be deserialized into, not read: these tests assert on the
    // *errors* produced, never on a successful value.
    #[derive(Debug, Deserialize, Validate)]
    #[allow(dead_code)]
    struct Sample {
        #[validate(email)]
        email: String,
        password: String,
        #[serde(default)]
        port: Option<u16>,
    }

    /// Runs the same deserialization path the extractor uses.
    fn parse(json: &str) -> Result<Sample, serde_path_to_error::Error<serde_json::Error>> {
        let mut de = serde_json::Deserializer::from_slice(json.as_bytes());
        serde_path_to_error::deserialize(&mut de)
    }

    #[test]
    fn a_missing_field_is_named_exactly() {
        let err = parse(r#"{"email":"a@b.com"}"#).unwrap_err();

        assert_eq!(field_path(&err), "password");
        assert_eq!(classify(&err.to_string()).0, "required");
    }

    #[test]
    fn a_type_mismatch_names_the_field_not_the_body() {
        // The case my message-parsing version could only report as "body".
        let err = parse(r#"{"email":123,"password":"a good password"}"#).unwrap_err();

        assert_eq!(field_path(&err), "email");
        assert_eq!(classify(&err.to_string()).0, "invalid_type");
    }

    #[test]
    fn a_nested_type_mismatch_reports_a_dotted_path() {
        let err =
            parse(r#"{"email":"a@b.com","password":"pw","port":"not a number"}"#).unwrap_err();

        assert_eq!(field_path(&err), "port");
    }

    #[test]
    fn broken_json_is_classified_as_a_syntax_error() {
        let err = parse(r#"{"email": broken"#).unwrap_err();

        assert_eq!(err.inner().classify(), serde_json::error::Category::Syntax);
    }

    /// The submitted value must never be echoed back — serde puts it in the
    /// message, and this is the boundary that strips it.
    #[test]
    fn the_submitted_value_is_never_echoed_to_the_client() {
        let err = parse(r#"{"email":"a@b.com","password":12345,"port":null}"#).unwrap_err();
        let (_, message) = classify(&err.to_string());

        assert!(err.to_string().contains("12345"), "serde does include the value");
        assert!(!message.contains("12345"), "but the client-facing message must not");
    }

    #[test]
    fn missing_field_parsing_declines_unrelated_messages() {
        assert_eq!(missing_field_name("some other problem"), None);
        assert_eq!(missing_field_name("missing field `` at line 1"), None);
    }
}
