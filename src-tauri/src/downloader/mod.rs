pub mod aria2c;
pub mod manager;
pub mod native;
pub mod notifications;

use crate::config::settings::ProxyMode;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadTask {
    pub id: String,
    pub url: String,
    pub output_dir: String,
    pub temp_path: String,
    pub final_path: String,
    pub filename: String,
    pub session_cookie: Option<String>,
    pub proxy_mode: ProxyMode,
    pub proxy_url: String,
    pub proxy_username: String,
    pub proxy_password: String,
    pub proxy_bypass_local: bool,
    pub connections: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interruption {
    Pause,
    Cancel,
}

#[derive(Debug)]
pub enum DownloadRunError {
    Interrupted(Interruption),
    Failed(String),
}

impl From<String> for DownloadRunError {
    fn from(value: String) -> Self {
        Self::Failed(value)
    }
}

pub struct DownloadControl {
    request: AtomicU8,
}

impl DownloadControl {
    pub fn new() -> Self {
        Self {
            request: AtomicU8::new(0),
        }
    }

    pub fn pause(&self) {
        self.request.store(1, Ordering::Release);
    }

    pub fn cancel(&self) {
        self.request.store(2, Ordering::Release);
    }

    pub fn interruption(&self) -> Option<Interruption> {
        match self.request.load(Ordering::Acquire) {
            1 => Some(Interruption::Pause),
            2 => Some(Interruption::Cancel),
            _ => None,
        }
    }
}

impl Default for DownloadControl {
    fn default() -> Self {
        Self::new()
    }
}
