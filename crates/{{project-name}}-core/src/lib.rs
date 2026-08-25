//! Domain logic: the rules that decide *what should happen*, kept separate
//! from *how it's persisted or served*.
//!
//! # The no-I/O rule
//!
//! This crate must not depend on `sea-orm`, `reqwest`, or `tokio::net`
//! (enforced by CI's `no-io-in-core` job, not just this comment). Callers pass
//! data in and receive decisions out; persisting those decisions is
//! `{{project-name}}-db`'s job, and performing them (handling a request,
//! calling another service) is `{{project-name}}-api`'s.
//!
//! If you find yourself reaching for a database connection or an HTTP client
//! in here, the logic belongs in one of those two crates instead — this one
//! stays testable without spinning up any fixtures.
//!
//! This is a template starting point: add your own modules here as your
//! domain logic grows (e.g. `pub mod pricing;`, `pub mod scheduling;`).

/// Delete this once real domain logic replaces it — it exists only to give
/// the crate a compiling, testable starting point.
pub fn placeholder() -> &'static str {
    "replace me with real domain logic"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_compiles_and_runs() {
        assert_eq!(placeholder(), "replace me with real domain logic");
    }
}
