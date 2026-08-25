//! The error contract every consumer of this API depends on.
//!
//! Consumers branch on `code` and never on `message`, so `ErrorCode` is a
//! closed enum with fixed serialized names. Renaming a variant's `rename`
//! string is a breaking API change; adding a variant is not.
//!
//! Add your own domain-specific variants here as they come up — this ships
//! with only the ones every service needs on day one.

use serde::{Deserialize, Serialize};

/// Stable, machine-readable error identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    // 400
    InvalidRequest,
    /// Body parsed but one or more fields failed validation. Carries
    /// `error.field_errors` so a client can mark the offending inputs.
    ValidationFailed,
    // 403
    Forbidden,
    // 404
    NotFound,
    // 409
    Conflict,
    // 422
    UnprocessableEntity,
    // 429
    RateLimited,
    // 5xx — the only variant that must never carry internal detail.
    Internal,
}

impl ErrorCode {
    /// HTTP status for this code. Kept next to the code itself so a new
    /// variant cannot be added without deciding its status.
    pub const fn status(self) -> u16 {
        match self {
            Self::InvalidRequest => 400,
            // 422, not 400: the body was well-formed JSON, but a value broke a
            // business rule. A client fixes these per-field rather than
            // rebuilding the request.
            Self::ValidationFailed => 422,
            Self::Forbidden => 403,
            Self::NotFound => 404,
            Self::Conflict => 409,
            Self::UnprocessableEntity => 422,
            Self::RateLimited => 429,
            Self::Internal => 500,
        }
    }

    /// Default human-readable message. Free to change — it is not the contract.
    pub const fn default_message(self) -> &'static str {
        match self {
            Self::InvalidRequest => "The request is malformed.",
            Self::ValidationFailed => "One or more fields are invalid.",
            Self::Forbidden => "You do not have permission to do that.",
            Self::NotFound => "Not found.",
            Self::Conflict => "The request conflicts with the current state.",
            Self::UnprocessableEntity => "The request could not be processed.",
            Self::RateLimited => "Too many requests. Try again shortly.",
            Self::Internal => "An internal error occurred.",
        }
    }
}

/// One field's validation failure.
///
/// `code` is the contract (`too_short`, `invalid_email`, `required`, …);
/// `message` is for humans and may change, exactly as with `ErrorCode`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldError {
    /// Dotted path to the field, e.g. `email` or `address.city`.
    pub field: String,
    pub code: String,
    pub message: String,
}

impl FieldError {
    pub fn new(
        field: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self { field: field.into(), code: code.into(), message: message.into() }
    }
}

/// The domain error every crate returns. The HTTP layer maps this onto a
/// response.
#[derive(Debug, thiserror::Error)]
#[error("{code:?}: {message}")]
pub struct Error {
    pub code: ErrorCode,
    pub message: String,
    /// Per-field failures. Empty for errors that are not about field contents.
    pub field_errors: Vec<FieldError>,
    /// Internal cause. Logged, never serialized to a client.
    #[source]
    pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl Error {
    pub fn new(code: ErrorCode) -> Self {
        Self {
            code,
            message: code.default_message().to_owned(),
            field_errors: Vec::new(),
            source: None,
        }
    }

    /// Builds a `validation_failed` error carrying per-field detail.
    pub fn validation(field_errors: Vec<FieldError>) -> Self {
        Self { field_errors, ..Self::new(ErrorCode::ValidationFailed) }
    }

    /// Overrides the human-readable message. Never put internal detail here —
    /// it is serialized to the client.
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    /// Attaches an internal cause for logs. Not serialized.
    pub fn with_source(mut self, source: impl std::error::Error + Send + Sync + 'static) -> Self {
        self.source = Some(Box::new(source));
        self
    }

    pub fn internal(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::new(ErrorCode::Internal).with_source(source)
    }

    pub const fn status(&self) -> u16 {
        self.code.status()
    }

    /// Builds the wire response. `Internal` never leaks `message` or `source`.
    pub fn to_envelope(&self, request_id: impl Into<String>) -> ErrorEnvelope {
        let message = if self.code == ErrorCode::Internal {
            ErrorCode::Internal.default_message().to_owned()
        } else {
            self.message.clone()
        };

        ErrorEnvelope {
            error: ErrorBody {
                code: self.code,
                message,
                request_id: request_id.into(),
                // Never echoed for internal errors: the field list is built
                // from request data and could restate something sensitive.
                field_errors: if self.code == ErrorCode::Internal {
                    Vec::new()
                } else {
                    self.field_errors.clone()
                },
            },
        }
    }
}

/// The exact JSON shape every consumer depends on: `{ "error": { code, message, request_id } }`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEnvelope {
    pub error: ErrorBody,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBody {
    pub code: ErrorCode,
    pub message: String,
    pub request_id: String,
    /// Per-field detail, present only for `validation_failed`.
    ///
    /// Omitted entirely when empty, so the envelope for every other error
    /// keeps the exact same shape — this is an additive change, not a
    /// breaking one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub field_errors: Vec<FieldError>,
}

/// Flattens `validator`'s nested error tree into a flat list of `FieldError`.
///
/// Nested and list fields become dotted/indexed paths (`address.city`,
/// `members[0].email`) so a client can map each entry to one input.
pub fn field_errors_from(errors: &validator::ValidationErrors) -> Vec<FieldError> {
    let mut collected = Vec::new();
    flatten_into("", errors, &mut collected);
    // Stable order: the same bad request must not produce a different
    // response each time, which would defeat response caching and confuse
    // tests.
    collected.sort_by(|a, b| a.field.cmp(&b.field).then_with(|| a.code.cmp(&b.code)));
    collected
}

fn flatten_into(prefix: &str, errors: &validator::ValidationErrors, out: &mut Vec<FieldError>) {
    use validator::ValidationErrorsKind;

    for (field, kind) in errors.errors() {
        let path = if prefix.is_empty() { field.to_string() } else { format!("{prefix}.{field}") };

        match kind {
            ValidationErrorsKind::Field(field_errors) => {
                for error in field_errors {
                    let message = error
                        .message
                        .as_ref()
                        .map(|m| m.to_string())
                        .unwrap_or_else(|| default_field_message(&error.code, &path));

                    out.push(FieldError::new(path.clone(), error.code.to_string(), message));
                }
            }
            ValidationErrorsKind::Struct(nested) => flatten_into(&path, nested, out),
            ValidationErrorsKind::List(indexed) => {
                for (index, nested) in indexed {
                    flatten_into(&format!("{path}[{index}]"), nested, out);
                }
            }
        }
    }
}

/// Fallback message for a rule that did not supply one.
///
/// Deliberately does not echo the submitted value: a validation response is a
/// place where a secret could otherwise be reflected back.
fn default_field_message(code: &str, field: &str) -> String {
    match code {
        "email" => "Must be a valid email address.".to_owned(),
        "length" => "Has an invalid length.".to_owned(),
        "range" => "Is out of range.".to_owned(),
        "required" | "required_nested" => "Is required.".to_owned(),
        "url" => "Must be a valid URL.".to_owned(),
        "regex" => "Is not in the expected format.".to_owned(),
        "must_match" => "Does not match.".to_owned(),
        _ => format!("`{field}` is invalid."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_serialize_as_snake_case_contract_strings() {
        let json = serde_json::to_string(&ErrorCode::NotFound).unwrap();
        assert_eq!(json, "\"not_found\"");
    }

    #[test]
    fn internal_errors_never_leak_message_or_cause() {
        let err = Error::internal(std::io::Error::other("db password is hunter2"))
            .with_message("connection to 10.0.0.5 failed");

        let envelope = err.to_envelope("req_1");
        assert_eq!(envelope.error.message, ErrorCode::Internal.default_message());
        assert!(!envelope.error.message.contains("hunter2"));
        assert!(!envelope.error.message.contains("10.0.0.5"));
    }

    #[test]
    fn non_internal_errors_keep_their_message() {
        let err = Error::new(ErrorCode::NotFound).with_message("no such record");
        assert_eq!(err.to_envelope("req_1").error.message, "no such record");
    }

    /// The envelope for every non-validation error must stay byte-identical
    /// across calls — `field_errors` is additive, not a change.
    #[test]
    fn field_errors_is_omitted_entirely_when_empty() {
        let err = Error::new(ErrorCode::Forbidden);
        let json = serde_json::to_value(err.to_envelope("req_1")).unwrap();

        assert!(json["error"].get("field_errors").is_none());
    }

    #[test]
    fn validation_errors_serialize_their_field_list() {
        let err = Error::validation(vec![
            FieldError::new("email", "email", "Must be a valid email address."),
            FieldError::new("password", "length", "Too short."),
        ]);

        assert_eq!(err.code, ErrorCode::ValidationFailed);
        assert_eq!(err.status(), 422);

        let json = serde_json::to_value(err.to_envelope("req_1")).unwrap();
        assert_eq!(json["error"]["field_errors"][0]["field"], "email");
        assert_eq!(json["error"]["field_errors"][1]["code"], "length");
    }

    /// An internal error must not echo a field list either: it is built from
    /// request data and could restate something sensitive.
    #[test]
    fn internal_errors_drop_any_field_errors() {
        let mut err = Error::new(ErrorCode::Internal);
        err.field_errors = vec![FieldError::new("password", "length", "Too short.")];

        assert!(err.to_envelope("req_1").error.field_errors.is_empty());
    }

    #[test]
    fn field_errors_are_returned_in_a_stable_order() {
        use validator::Validate;

        #[derive(Validate)]
        struct Sample {
            #[validate(email)]
            email: String,
            #[validate(length(min = 8))]
            password: String,
        }

        let sample = Sample { email: "nope".to_owned(), password: "short".to_owned() };
        let errors = sample.validate().unwrap_err();

        // Sorted by field, so the same bad request always yields the same body.
        let fields: Vec<_> = field_errors_from(&errors).into_iter().map(|e| e.field).collect();
        assert_eq!(fields, ["email", "password"]);
    }
}
