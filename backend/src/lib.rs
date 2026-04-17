pub mod api;
pub mod cc_hook_socket;
pub mod db;
pub mod drivers;
pub mod events;
pub mod ringbuf;
pub mod session;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use db::Db;
use drivers::OobMessage;
use events::EventBus;
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub event_bus: Arc<EventBus>,
    pub ring_dir: PathBuf,
    pub hook_channels: Arc<Mutex<HashMap<String, mpsc::Sender<OobMessage>>>>,
}
