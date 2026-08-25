//! Server assembly: middleware, binding, and graceful shutdown.

use axum::{http::StatusCode, Router};
use tokio::net::TcpListener;
use tower_http::{
    request_id::MakeRequestUuid, timeout::TimeoutLayer, trace::TraceLayer, ServiceBuilderExt,
};

use crate::{config::Config, routes, state::AppState};

/// Wraps the route table in the middleware stack.
///
/// Order matters: request-id generation is outermost so the id exists before
/// tracing records it, and `propagate_x_request_id` runs innermost-last so the
/// id is copied onto the response. `request_id` appears in the error envelope,
/// which is what makes a client-reported id findable in logs.
pub fn build(state: AppState, config: &Config) -> Router {
    let middleware = tower::ServiceBuilder::new()
        .set_x_request_id(MakeRequestUuid)
        .layer(TraceLayer::new_for_http())
        // A request that outlives this is a stuck dependency, not slow work.
        .layer(TimeoutLayer::with_status_code(
            StatusCode::GATEWAY_TIMEOUT,
            config.request_timeout,
        ))
        .propagate_x_request_id();

    routes::router(state)
        // Innermost first: `render` needs the id that `propagate` puts
        // in the extensions, so it is layered after it.
        .layer(axum::middleware::from_fn(crate::error::render))
        .layer(axum::middleware::from_fn(crate::request_id::propagate))
        .layer(middleware)
}

/// Binds the listener and serves until a shutdown signal arrives.
///
/// After the signal, in-flight requests get `config.shutdown_timeout` to finish.
/// The bound matters: without it a single stuck handler keeps the process alive
/// indefinitely, and the orchestrator eventually SIGKILLs it — turning a clean
/// deploy into a hard kill that severs every other in-flight request too.
pub async fn serve(router: Router, config: &Config) -> std::io::Result<()> {
    let listener = TcpListener::bind(config.listen_address).await?;
    let bound = listener.local_addr()?;

    tracing::info!(address = %bound, "listening");

    // The deadline must start when the signal arrives, not when the server
    // starts — wrapping the whole future in a timeout would kill a healthy
    // process after `shutdown_timeout` of normal operation.
    let (drain_started, drain_deadline) = tokio::sync::oneshot::channel::<()>();
    let grace = config.shutdown_timeout;

    let shutdown = async move {
        shutdown_signal().await;
        // Signals the watchdog below that draining has begun.
        let _ = drain_started.send(());
    };

    let watchdog = async move {
        if drain_deadline.await.is_err() {
            // Sender dropped: the server exited without a signal, so there is
            // nothing left to wait for.
            return;
        }
        tokio::time::sleep(grace).await;
        tracing::warn!(
            timeout = ?grace,
            "in-flight requests did not finish before the shutdown deadline; exiting anyway"
        );
    };

    let server = axum::serve(listener, router).with_graceful_shutdown(shutdown);

    tokio::select! {
        result = server => result,
        // Fires only once the grace period after the signal has elapsed.
        () = watchdog => Ok(()),
    }
}

/// Resolves on SIGTERM (orchestrator stop) or Ctrl-C (local development).
///
/// Without this, a deploy severs in-flight requests mid-response instead of
/// letting them finish.
async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(err) = tokio::signal::ctrl_c().await {
            tracing::error!(error = %err, "failed to listen for ctrl-c");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(err) => tracing::error!(error = %err, "failed to listen for SIGTERM"),
        }
    };

    // Windows has no SIGTERM; Ctrl-C is the only stop signal there.
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => tracing::info!("received ctrl-c, shutting down"),
        () = terminate => tracing::info!("received SIGTERM, shutting down"),
    }
}
