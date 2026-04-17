pub mod api;
pub mod cc_hook_socket;
pub mod config;
pub mod db;
pub mod drivers;
pub mod events;
pub mod pty;
pub mod push;
pub mod raw_fd_master;
pub mod ringbuf;
pub mod session;
pub mod supervisor_client;
pub mod supervisor_protocol;
pub mod ws;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use db::Db;
use drivers::OobMessage;
use events::EventBus;
use supervisor_client::SupervisorClient;
use tokio::sync::mpsc;

pub type SessionRegistry = Arc<RwLock<HashMap<String, pty::ActiveSession>>>;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub event_bus: Arc<EventBus>,
    pub ring_dir: PathBuf,
    pub hook_channels: Arc<Mutex<HashMap<String, mpsc::Sender<OobMessage>>>>,
    pub sessions: SessionRegistry,
    pub supervisor: Option<SupervisorClient>,
}
