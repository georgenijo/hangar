pub mod api;
pub mod session;
pub mod supervisor;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;

pub struct AppState {
    pub supervisor_connected: Arc<AtomicBool>,
    pub started_at: Instant,
    pub supervisor: Option<Arc<supervisor::SupervisorHandle>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            supervisor_connected: Arc::new(AtomicBool::new(false)),
            started_at: Instant::now(),
            supervisor: None,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
