//! Postgres access: connection pool, SeaORM entities, and repositories.
//!
//! This crate owns every detail of *how* data is stored.
//! `{{project-name}}-core` decides what should happen; this crate makes it
//! durable.

pub mod connection;
pub mod entities;
pub mod repository;

pub use connection::{Database, DbConfig};

/// A real Postgres connection for tests that need actual database behaviour —
/// constraints, joins, transactions — rather than a hand-built fake.
///
/// Every repository test in this crate should share this rather than opening
/// its own pool, and relies on the schema already being migrated
/// (`cargo run -p {{project-name}}-migration -- up`) — the same
/// `DATABASE_URL` the app itself reads.
#[cfg(test)]
pub(crate) mod test_support {
    use crate::{Database, DbConfig};

    pub async fn db() -> Database {
        let _ = dotenvy::dotenv();
        let url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set to run this crate's integration tests");

        Database::connect(&DbConfig::new(url))
            .await
            .expect("failed to connect to the test database — is it migrated and running?")
    }
}
