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
}
