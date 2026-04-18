use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;
use tracing::{info, warn};

use hangard::{api, db::Db, events::EventBus, session::SessionState, AppState};

#[tokio::main]
async fn main() -> Result<()> {
    let start_time = Instant::now();
    tracing_subscriber::fmt::init();

    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("no home dir"))?;
    let state_dir = home.join(".local/state/hangar");
    std::fs::create_dir_all(&state_dir)?;

    let sessions_dir = state_dir.join("sessions");
    std::fs::create_dir_all(&sessions_dir)?;

    let db = Db::new(Some(state_dir.join("hangar.db"))).await?;

    let event_bus = Arc::new(EventBus::new());
    let sessions_registry: hangard::SessionRegistry = Arc::new(RwLock::new(HashMap::new()));

    // Attempt supervisor connection and session recovery
    let sock_path = hangard::supervisor_protocol::supervisor_sock_path();
    let supervisor = match hangard::supervisor_client::SupervisorClient::connect(&sock_path) {
        Ok(client) => {
            info!("connected to supervisor at {:?}", sock_path);
            Some(client)
        }
        Err(e) => {
            warn!("supervisor not available ({e}), sessions won't survive restart");
            None
        }
    };

    if let Some(ref sup) = supervisor {
        // Recover live sessions from supervisor
        match recover_sessions(
            sup,
            db.pool(),
            &sessions_dir,
            &event_bus,
            &sessions_registry,
        )
        .await
        {
            Ok(n) => info!("recovered {} sessions from supervisor", n),
            Err(e) => warn!("session recovery failed: {e}"),
        }
    } else {
        // No supervisor: mark all stale sessions as exited (original behaviour)
        let marked = hangard::session::Session::mark_active_as_exited(db.pool()).await?;
        if marked > 0 {
            info!("marked {} stale sessions as exited", marked);
        }
    }

    let config = hangard::config::load().unwrap_or_else(|e| {
        warn!("config load failed: {e}, using defaults");
        hangard::config::HangarConfig::default()
    });

    let sandbox_manager = if config.sandbox.enabled {
        let mgr = hangard::sandbox::SandboxManager::new(
            config.sandbox.overlay_base.clone(),
            config.sandbox.restic_repo.clone(),
        );
        if let Err(e) = mgr.startup_cleanup(db.pool()).await {
            warn!("sandbox startup_cleanup failed: {e}");
        }
        if let Err(e) = mgr.ensure_restic_repo().await {
            warn!("ensure_restic_repo failed: {e}");
        }
        Some(Arc::new(mgr))
    } else {
        None
    };

    let mut logs_hub = hangard::logs::LogsHub::new(&config.logs, &sessions_dir);
    logs_hub.start();
    let logs_hub = Arc::new(logs_hub);

    let app_state = AppState {
        db,
        event_bus,
        ring_dir: sessions_dir,
        hook_channels: Arc::new(Mutex::new(HashMap::new())),
        sessions: sessions_registry,
        supervisor,
        start_time,
        sandbox_manager,
        logs: logs_hub,
    };

    tokio::spawn(hangard::push::run(
        app_state.event_bus.clone(),
        app_state.db.clone(),
        config.push,
    ));
    info!("push task spawned");

    let router = api::router(app_state);

    let port: u16 = std::env::var("HANGAR_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let addr = format!("127.0.0.1:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("listening on {}", addr);

    axum::serve(listener, router).await?;

    Ok(())
}

async fn recover_sessions(
    supervisor: &hangard::supervisor_client::SupervisorClient,
    pool: &sqlx::SqlitePool,
    sessions_dir: &std::path::Path,
    event_bus: &Arc<EventBus>,
    registry: &hangard::SessionRegistry,
) -> Result<usize> {
    use hangard::drivers::DriverRegistry;
    use hangard::pty;
    use hangard::session::Session;

    let sup_sessions = supervisor.list().await?;
    let db_sessions = Session::list(pool).await?;

    let live_in_sup: std::collections::HashSet<String> = sup_sessions
        .iter()
        .filter(|s| s.alive)
        .map(|s| s.session_id.clone())
        .collect();

    let driver_registry = DriverRegistry::new();
    let mut reattached = 0usize;

    for session in &db_sessions {
        if matches!(session.state, SessionState::Exited) {
            continue;
        }
        let sid_str = session.id.to_string();
        if live_in_sup.contains(&sid_str) {
            let driver = match driver_registry.create(session.kind.driver_kind()) {
                Some(d) => d,
                None => {
                    warn!("unknown driver kind for session {sid_str}, marking exited");
                    Session::update_state(pool, &session.id, SessionState::Exited).await?;
                    continue;
                }
            };

            match supervisor.attach_fd(&sid_str).await {
                Ok((_, fd)) => {
                    match pty::reattach(
                        session.id.clone(),
                        fd,
                        driver,
                        sessions_dir.to_path_buf(),
                        Arc::clone(event_bus),
                        pool.clone(),
                    ) {
                        Ok(active) => {
                            registry.write().unwrap().insert(sid_str.clone(), active);
                            info!("reattached session {sid_str}");
                            reattached += 1;
                        }
                        Err(e) => {
                            warn!("reattach failed for {sid_str}: {e}");
                            Session::update_state(pool, &session.id, SessionState::Exited).await?;
                        }
                    }
                }
                Err(e) => {
                    warn!("attach_fd failed for {sid_str}: {e}");
                    Session::update_state(pool, &session.id, SessionState::Exited).await?;
                }
            }
        } else {
            info!("session {sid_str} not in supervisor, marking exited");
            Session::update_state(pool, &session.id, SessionState::Exited).await?;
        }
    }

    Ok(reattached)
}
