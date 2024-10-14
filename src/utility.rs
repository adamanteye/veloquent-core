/// Used in axum server to perform graceful shutdown.
///
/// Adapted from [axum graceful-shutdown](https://github.com/tokio-rs/axum/tree/main/examples/graceful-shutdown) with non-unix part removed.
pub(super) async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::event!(tracing::Level::INFO, "gracefully shutting down");
}

use {once_cell::sync::Lazy, regex::Regex};

pub(super) fn good_email(regex: &str) -> bool {
    static RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r"^([a-z0-9_+]([a-z0-9_+.]*[a-z0-9_+])?)@([a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,6})",
        )
        .unwrap()
    });
    RE.is_match(regex)
}
