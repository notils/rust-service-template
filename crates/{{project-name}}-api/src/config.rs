//! Configuration, read once from the environment at startup.
//!
//! Validation collects *every* problem before returning, so a fresh checkout
//! reports all missing/invalid variables at once instead of one per restart.
//! Add your own fields here as your service needs them — a present-but-
//! unparseable value should be an error, not a silent fallback to a default
//! (see `parse_or_default` below): a typo in a TTL or a port must not
//! quietly restore the default and hide the mistake.

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

use {{crate_name}}_db::DbConfig;

/// Every failure found while reading the environment.
#[derive(Debug, thiserror::Error)]
#[error("invalid configuration:\n{}", .0.join("\n"))]
pub struct ConfigError(Vec<String>);

#[derive(Debug, Clone)]
pub struct Config {
    pub listen_address: SocketAddr,
    pub database: DbConfig,
    /// Grace period for in-flight requests during shutdown.
    pub shutdown_timeout: Duration,
    /// A request exceeding this is a stuck dependency, not slow work.
    pub request_timeout: Duration,
}

impl Config {
    /// Reads and validates configuration from the process environment.
    pub fn from_env() -> Result<Self, ConfigError> {
        let mut errors = Vec::new();

        let database_url = require("DATABASE_URL", &mut errors);
        let host = parse_or_default("HOST", IpAddr::V4(Ipv4Addr::UNSPECIFIED), &mut errors);
        let port = parse_or_default("PORT", 8080_u16, &mut errors);
        let max_connections = parse_or_default("DATABASE_MAX_CONNECTIONS", 10_u32, &mut errors);
        let log_statements = parse_or_default("DATABASE_LOG_STATEMENTS", false, &mut errors);
        let request_timeout = parse_or_default("REQUEST_TIMEOUT_SECONDS", 30_u64, &mut errors);

        if !errors.is_empty() {
            return Err(ConfigError(errors));
        }

        let mut database = DbConfig::new(database_url.unwrap_or_default());
        database.max_connections = max_connections;
        database.log_statements = log_statements;

        Ok(Self {
            listen_address: SocketAddr::new(host, port),
            database,
            shutdown_timeout: Duration::from_secs(15),
            request_timeout: Duration::from_secs(request_timeout),
        })
    }
}

fn require(key: &str, errors: &mut Vec<String>) -> Option<String> {
    match std::env::var(key) {
        Ok(value) if !value.trim().is_empty() => Some(value),
        Ok(_) => {
            errors.push(format!("  {key} is set but empty"));
            None
        }
        Err(_) => {
            errors.push(format!("  {key} is required but not set"));
            None
        }
    }
}

/// Reads an optional variable, falling back to `default` when absent. A present
/// but unparseable value is an error rather than a silent fallback.
fn parse_or_default<T>(key: &str, default: T, errors: &mut Vec<String>) -> T
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match std::env::var(key) {
        Err(_) => default,
        Ok(raw) => match raw.trim().parse::<T>() {
            Ok(value) => value,
            Err(err) => {
                errors.push(format!("  {key}: {err} (got {raw:?})"));
                default
            }
        },
    }
}
