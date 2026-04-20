use anyhow::Result;
use clap::Parser;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;
use tracing::{info, warn};

use hangard::{
    api,
    db::Db,
    events::{EventBus, EventStore},
    session::SessionState,
    AppState,
};

/// Hangar backend daemon - manages development sessions with PTY support
#[derive(Parser, Debug)]
#[command(name = "hangard")]
#[command(about = "Hangar backend daemon", long_about = None)]
struct Args {
    /// Port to bind the HTTP server (can also be set via HANGAR_PORT env var)
    #[arg(long, default_value_t = 3000)]
    port: u16,

    /// Path to SQLite database file
    #[arg(long)]
    db_path: Option<PathBuf>,

    /// Path to supervisor socket
    #[arg(long)]
    supervisor_sock: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let start_time = Instant::now();
    tracing_subscriber::fmt::init();

    let state_dir = hangard::supervisor_protocol::hangar_state_dir();
    std::fs::create_dir_all(&state_dir)?;

    let sessions_dir = state_dir.join("sessions");
    std::fs::create_dir_all(&sessions_dir)?;

    let db_path = args.db_path.unwrap_or_else(|| state_dir.join("hangar.db"));
    let db = Db::new(Some(db_path)).await?;

    let backfilled = EventStore::backfill_fts(db.pool()).await?;
    if backfilled > 0 {
        info!("backfilled FTS index for {} events", backfilled);
    }

    let event_bus = Arc::new(EventBus::new());
    let sessions_registry: hangard::SessionRegistry = Arc::new(RwLock::new(HashMap::new()));

    // Attempt supervisor connection and session recovery
    let sock_path = args
        .supervisor_sock
        .unwrap_or_else(|| hangard::supervisor_protocol::supervisor_sock_path());
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

    // Event persister: subscribe to the event bus and write every event to DB
    // so /events and downstream UIs can read history after the fact.
    {
        let pool = app_state.db.pool().clone();
        let mut rx = app_state.event_bus.subscribe();
        tokio::spawn(async move {
            while let Ok((session_id, event)) = rx.recv().await {
                if let Err(e) =
                    hangard::events::EventStore::insert(&pool, &session_id, &event).await
                {
                    // #63: FK 787 fires when the session row is deleted before a trailing
                    // event is flushed. Expected — downgrade to debug and continue.
                    let is_fk = matches!(
                        e.downcast_ref::<sqlx::Error>(),
                        Some(sqlx::Error::Database(dbe)) if dbe.code().as_deref() == Some("787")
                    );
                    if is_fk {
                        tracing::debug!("event for deleted session dropped (sid={})", session_id);
                    } else {
                        tracing::warn!("event persist failed (sid={}): {}", session_id, e);
                    }
                }
            }
        });
        info!("event persister spawned");
    }

    let router = api::router(app_state);

    // Priority: HANGAR_PORT env var > --port CLI arg > default (3000)
    let port: u16 = std::env::var("HANGAR_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(args.port);

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
