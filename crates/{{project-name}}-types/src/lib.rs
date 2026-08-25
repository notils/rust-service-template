//! Shared types for every {{project-name}} crate.
//!
//! This crate has no I/O dependencies and sits at the bottom of the dependency
//! graph — anything here is safe to use from `{{project-name}}-core` without
//! violating the no-I/O rule (see docs/architecture.md).

pub mod error;
pub mod ids;

pub use error::{Error, ErrorBody, ErrorCode, ErrorEnvelope, FieldError, field_errors_from};
