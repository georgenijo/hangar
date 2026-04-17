use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use hangar_backend::supervisor::SupervisorHandle;
use hangar_backend::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!("hangard starting");

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let socket_path = PathBuf::from(format!("{}/.local/state/hangar/supervisor.sock", home));

    let mut app_state = AppState::new();

    match SupervisorHandle::connect(&socket_path).await {
        Ok(handle) => {
            tracing::info!("connected to supervisor");
            app_state.supervisor_connected.store(true, Ordering::SeqCst);

            // List existing sessions
            match handle.list().await {
                Ok(sessions) => {
                    tracing::info!(count = sessions.len(), "supervisor reports active sessions");
                    for s in &sessions {
                        tracing::info!(
                            id = %s.id,
                            slug = %s.slug,
                            pid = s.pid,
                            running = s.running,
                            "existing session"
                        );
                    }
                }
                Err(e) => tracing::warn!("failed to list sessions from supervisor: {}", e),
            }

            app_state.supervisor = Some(handle);
        }
        Err(e) => {
            tracing::warn!(
                "supervisor unreachable: {} — sessions will not survive restart",
                e
            );
        }
    }

    let state = Arc::new(app_state);
    let app = hangar_backend::api::router().with_state(state);

    let bind_addr = "127.0.0.1:3000";
    tracing::info!("listening on {}", bind_addr);
    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
