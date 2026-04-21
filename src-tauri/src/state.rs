use std::sync::Arc;
use std::time::Duration;

use firmware_index::Index;
use tokio::sync::RwLock;

/// Process-wide state shared across Tauri command handlers.
pub struct AppState {
    pub http: reqwest::Client,
    pub index: Arc<RwLock<Option<Index>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .user_agent(concat!("walksnail-updater/", env!("CARGO_PKG_VERSION")))
                .timeout(Duration::from_secs(60))
                .build()
                .expect("reqwest client builds"),
            index: Arc::new(RwLock::new(None)),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
