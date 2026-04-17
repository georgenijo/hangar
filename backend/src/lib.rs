pub mod api;
pub mod db;
pub mod events;
pub mod ringbuf;
pub mod session;

use std::path::PathBuf;
use std::sync::Arc;

use db::Db;
use events::EventBus;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub event_bus: Arc<EventBus>,
    pub ring_dir: PathBuf,
}
