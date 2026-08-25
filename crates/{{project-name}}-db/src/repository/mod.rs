//! Repositories: the only place SQL is written.
//!
//! Each takes a `Database` handle (cheap to clone) and exposes intent-named
//! methods rather than query builders, so callers cannot accidentally
//! construct a query that bypasses a soft-delete or tenancy filter.
//!
//! Add one `pub mod` per aggregate/table group as you build them out, and
//! re-export its public repository type(s) here.
//!
//! Test each repository against a real Postgres, not a mock — constraints,
//! joins, and transactions are exactly what a mock can't catch. When you add
//! the first repository test, add `tokio` and `dotenvy` under
//! `[dev-dependencies]` in this crate's `Cargo.toml`, then add this alongside
//! it in `lib.rs`:
//!
//! ```ignore
//! #[cfg(test)]
//! pub(crate) mod test_support {
//!     use crate::{Database, DbConfig};
//!
//!     pub async fn db() -> Database {
//!         let _ = dotenvy::dotenv();
//!         let url = std::env::var("DATABASE_URL")
//!             .expect("DATABASE_URL must be set to run this crate's integration tests");
//!
//!         Database::connect(&DbConfig::new(url))
//!             .await
//!             .expect("failed to connect to the test database — is it migrated and running?")
//!     }
//! }
//! ```
