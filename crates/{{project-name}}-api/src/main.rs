//! Entrypoint.
//!
//! Startup is deliberately fail-fast: configuration and the database are both
//! validated before the listener binds, so a misconfigured instance never
//! reports itself as healthy.

use std::process::ExitCode;

use {{crate_name}}_api::{AppState, Config, server, telemetry};
use {{crate_name}}_db::Database;

#[tokio::main]
async fn main() -> ExitCode {
    // Development convenience only; in deployment the environment is injected.
    let _ = dotenvy::dotenv();

    // Handled before telemetry so a probe does not emit a log line every 30
    // seconds for the lifetime of the container.
    if std::env::args().any(|arg| arg == "--health-check") {
        return health_check().await;
    }

    telemetry::init();

    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            // `Display` on the error chain, not `Debug` — a panic-style dump is
            // unreadable in a deploy log.
            tracing::error!("startup failed: {err}");
            let mut source = std::error::Error::source(&*err);
            while let Some(cause) = source {
                tracing::error!("  caused by: {cause}");
                source = cause.source();
            }
            ExitCode::FAILURE
        }
    }
}

/// Probes the local liveness endpoint and exits 0 or 1.
///
/// Exists so the container image needs no `curl` or `wget` — the runtime stage
/// installs only ca-certificates, and adding a shell utility purely for probing
/// would widen the attack surface for no benefit.
///
/// ⚠️ Hits `/health`, never `/health/ready`. Readiness touches the database, and
/// a container restart is the wrong response to a transient database blip.
async fn health_check() -> ExitCode {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_owned());
    let url = format!("http://127.0.0.1:{port}/health");

    match probe(&url).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            // Goes to stderr, not the tracing subscriber, which is not installed
            // on this path.
            eprintln!("health check failed: {err}");
            ExitCode::FAILURE
        }
    }
}

/// Minimal HTTP/1.1 GET over a raw socket.
///
/// A hand-rolled request rather than an HTTP client dependency: the probe needs
/// one status line from localhost, and `reqwest` would pull a TLS stack into a
/// binary that otherwise has none.
async fn probe(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

    let authority = url.trim_start_matches("http://");
    let (host_port, path) = authority.split_once('/').unwrap_or((authority, ""));

    let mut stream = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        tokio::net::TcpStream::connect(host_port),
    )
    .await??;

    let request = format!("GET /{path} HTTP/1.1\r\nHost: {host_port}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).await?;

    let mut response = Vec::new();
    tokio::time::timeout(
        std::time::Duration::from_secs(2),
        stream.read_to_end(&mut response),
    )
    .await??;

    let head = String::from_utf8_lossy(&response);
    let status = head.lines().next().unwrap_or_default();

    if status.contains(" 200") {
        Ok(())
    } else {
        Err(format!("unexpected status: {status}").into())
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env()?;

    let db = Database::connect(&config.database).await?;
    tracing::info!("database connected");

    let state = AppState::new(db.clone(), config.clone());
    let router = server::build(state, &config);

    server::serve(router, &config).await?;

    // Reached only after graceful shutdown completes.
    db.close().await?;
    tracing::info!("shutdown complete");

    Ok(())
}
