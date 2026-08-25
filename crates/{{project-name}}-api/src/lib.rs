//! HTTP surface: configuration, routes, middleware, and error mapping.
//!
//! Exposed as a library so integration tests can build the same router the
//! binary serves, rather than testing a separate approximation of it.

pub mod config;
pub mod error;
pub mod request_id;
pub mod routes;
pub mod server;
pub mod state;
pub mod telemetry;
pub mod validation;

pub use config::Config;
pub use error::ApiError;
pub use request_id::RequestId;
pub use state::AppState;
