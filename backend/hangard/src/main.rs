mod api;
mod pty;
mod session;
mod ws;

use api::AppState;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let app_state = AppState::new();

    let app = axum::Router::new()
        .merge(api::router())
        .merge(ws::router())
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(app_state);

    let listener = TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("bind 127.0.0.1:3000");

    tracing::info!("hangard listening on 127.0.0.1:3000");
    axum::serve(listener, app).await.expect("server error");
}
