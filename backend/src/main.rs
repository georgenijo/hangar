use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::info;

use hangard::{api, db::Db, events::EventBus, AppState};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("no home dir"))?;
    let state_dir = home.join(".local/state/hangar");
    std::fs::create_dir_all(&state_dir)?;

    let sessions_dir = state_dir.join("sessions");
    std::fs::create_dir_all(&sessions_dir)?;

    let db = Db::new(Some(state_dir.join("hangar.db"))).await?;

    let marked = hangard::session::Session::mark_active_as_exited(db.pool()).await?;
    if marked > 0 {
        info!("marked {} stale sessions as exited", marked);
    }

    let event_bus = Arc::new(EventBus::new());

    let app_state = AppState {
        db,
        event_bus,
        ring_dir: sessions_dir,
        hook_channels: Arc::new(Mutex::new(HashMap::new())),
    };

    let router = api::router(app_state);

    let port: u16 = std::env::var("HANGAR_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(4321);

    let addr = format!("127.0.0.1:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("listening on {}", addr);

    axum::serve(listener, router).await?;

    Ok(())
}
