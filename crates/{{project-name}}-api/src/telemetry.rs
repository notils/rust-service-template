//! Structured logging.

use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Installs the global subscriber.
///
/// `RUST_LOG` controls verbosity. JSON output is used when `LOG_FORMAT=json` so
/// deployed logs are machine-parseable, while local runs stay human-readable.
///
/// Call once, before anything worth logging happens.
pub fn init() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("info,{{crate_name}}_api=debug,{{crate_name}}_db=debug")
    });

    let json = std::env::var("LOG_FORMAT").is_ok_and(|format| format.eq_ignore_ascii_case("json"));

    let registry = tracing_subscriber::registry().with(filter);

    if json {
        registry
            .with(fmt::layer().json().flatten_event(true))
            .init();
    } else {
        registry.with(fmt::layer().compact()).init();
    }
}
