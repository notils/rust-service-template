//! The Postgres connection pool.

use std::time::Duration;

use {{crate_name}}_types::{Error, ErrorCode};
use sea_orm::{ConnectOptions, DatabaseConnection};

/// Pool tuning. Defaults suit a single API instance; raise `max_connections`
/// only alongside the ceiling of the managed Postgres, since every instance
/// holds its own pool.
#[derive(Debug, Clone)]
pub struct DbConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout: Duration,
    pub acquire_timeout: Duration,
    /// Recycles idle connections so a silently dropped TCP connection (common
    /// behind a managed-Postgres proxy) does not linger in the pool.
    pub idle_timeout: Duration,
    pub max_lifetime: Duration,
    /// Emits every SQL statement at the configured level. Leave off outside
    /// local debugging: statements can carry personal data into logs.
    pub log_statements: bool,
}

impl DbConfig {
    /// Builds a config from a connection URL, leaving pool settings at defaults.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            max_connections: 10,
            min_connections: 1,
            connect_timeout: Duration::from_secs(5),
            acquire_timeout: Duration::from_secs(5),
            idle_timeout: Duration::from_secs(600),
            max_lifetime: Duration::from_secs(1800),
            log_statements: false,
        }
    }
}

/// A handle to the Postgres pool.
///
/// `Clone` is cheap and does *not* open a new pool: `DatabaseConnection` is
/// already an internally reference-counted handle, exactly like `sqlx::PgPool`.
/// That is why this type needs no `Arc` wrapper to be shared as axum state.
#[derive(Debug, Clone)]
pub struct Database {
    conn: DatabaseConnection,
}

impl Database {
    /// Opens the pool and verifies it with a round-trip, so a bad URL or an
    /// unreachable server fails at startup rather than on the first request.
    pub async fn connect(config: &DbConfig) -> Result<Self, Error> {
        let mut options = ConnectOptions::new(&config.url);
        options
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .connect_timeout(config.connect_timeout)
            .acquire_timeout(config.acquire_timeout)
            .idle_timeout(config.idle_timeout)
            .max_lifetime(config.max_lifetime)
            .sqlx_logging(config.log_statements);

        let conn = sea_orm::Database::connect(options).await.map_err(|err| {
            // The URL carries the password, so it must never reach the message.
            Error::new(ErrorCode::Internal)
                .with_message("Could not connect to the database.")
                .with_source(err)
        })?;

        let db = Self { conn };
        db.ping().await?;

        Ok(db)
    }

    /// The pooled connection, for entities and repositories.
    pub const fn conn(&self) -> &DatabaseConnection {
        &self.conn
    }

    /// Checks that a pooled connection is still alive. Backs the readiness probe.
    pub async fn ping(&self) -> Result<(), Error> {
        self.conn.ping().await.map_err(|err| {
            Error::new(ErrorCode::Internal)
                .with_message("The database is not reachable.")
                .with_source(err)
        })
    }

    /// Closes the pool, waiting for checked-out connections to return. Called
    /// on shutdown so in-flight transactions are not cut off mid-statement.
    pub async fn close(self) -> Result<(), Error> {
        self.conn.close().await.map_err(Error::internal)
    }
}
