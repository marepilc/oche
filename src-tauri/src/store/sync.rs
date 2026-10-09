//! Background task that pushes queued local changes to the remote database.

use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::sync::{Mutex, Notify};

use super::local::{Local, OutboxItem};
use super::remote::{self, Remote, RemoteConfig};

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum SyncStatus {
    /// No remote database configured.
    Off,
    /// Everything is on the remote database.
    Synced,
    /// Changes are being pushed.
    Pushing { pending: i64 },
    /// The last attempt failed; it is retried later.
    Error { pending: i64, message: String },
}

pub struct Sync {
    pub local: Arc<Local>,
    pub config: Mutex<RemoteConfig>,
    pool: Mutex<Option<Remote>>,
    status: std::sync::Mutex<SyncStatus>,
    wake: Notify,
}

const IDLE: Duration = Duration::from_secs(60);
const RETRY: Duration = Duration::from_secs(15);

impl Sync {
    pub fn new(local: Arc<Local>, config: RemoteConfig) -> Self {
        local.set_tracking(config.enabled);
        Self {
            local,
            config: Mutex::new(config),
            pool: Mutex::new(None),
            status: std::sync::Mutex::new(SyncStatus::Off),
            wake: Notify::new(),
        }
    }

    pub fn status(&self) -> SyncStatus {
        self.status.lock().unwrap().clone()
    }

    /// Pushes pending changes now instead of at the next interval.
    pub fn wake(&self) {
        self.wake.notify_one();
    }

    /// Switches to a new remote database (or none), using an already prepared pool.
    pub async fn switch(&self, config: RemoteConfig, pool: Option<Remote>) {
        self.local.set_tracking(config.enabled);
        *self.config.lock().await = config;
        if let Some(old) = std::mem::replace(&mut *self.pool.lock().await, pool) {
            old.close().await;
        }
        self.wake();
    }

    fn set_status(&self, app: &AppHandle, status: SyncStatus) {
        let mut current = self.status.lock().unwrap();
        if *current != status {
            *current = status.clone();
            let _ = app.emit("sync-status", status);
        }
    }

    async fn pool(&self, config: &RemoteConfig) -> Result<Remote, String> {
        let mut slot = self.pool.lock().await;
        if let Some(pool) = slot.as_ref() {
            return Ok(pool.clone());
        }
        let password = remote::stored_password(config);
        let pool = Remote::connect(config, password.as_deref()).await?;
        pool.prepare().await?;
        *slot = Some(pool.clone());
        Ok(pool)
    }

    async fn push(&self, pool: &Remote, item: &OutboxItem) -> Result<(), String> {
        let local = &self.local;
        let err = |e: sqlx::Error| e.to_string();
        match (item.entity.as_str(), item.op.as_str()) {
            ("player", _) => {
                if let Some(p) = local.player(&item.entity_id).await.map_err(err)? {
                    pool.push_player(&p).await.map_err(err)?;
                }
            }
            ("game", "delete") => pool.delete_game(&item.entity_id).await.map_err(err)?,
            ("game", _) => {
                if let Some(g) = local.load_game(&item.entity_id).await.map_err(err)? {
                    let mut players = Vec::new();
                    for id in &g.players {
                        players.extend(local.player(id).await.map_err(err)?);
                    }
                    pool.push_game(&g, &players).await.map_err(err)?;
                }
            }
            _ => {}
        }
        local.pushed(item).await.map_err(err)
    }

    /// One round: push everything pending. Returns how long to wait before the next round.
    async fn round(&self, app: &AppHandle) -> Duration {
        let config = self.config.lock().await.clone();
        if !config.enabled {
            self.set_status(app, SyncStatus::Off);
            return IDLE;
        }
        let pending = self.local.pending_count().await.unwrap_or(0);
        let fail = |message: String| {
            self.set_status(app, SyncStatus::Error { pending, message });
            RETRY
        };
        let items = match self.local.pending().await {
            Ok(items) => items,
            Err(e) => return fail(e.to_string()),
        };
        if items.is_empty() {
            self.set_status(app, SyncStatus::Synced);
            return IDLE;
        }
        let pool = match self.pool(&config).await {
            Ok(pool) => pool,
            Err(e) => return fail(e),
        };
        for (i, item) in items.iter().enumerate() {
            self.set_status(app, SyncStatus::Pushing { pending: (items.len() - i) as i64 });
            if let Err(e) = self.push(&pool, item).await {
                // A broken connection is rebuilt on the next round.
                *self.pool.lock().await = None;
                return fail(e);
            }
        }
        // Changes made while pushing are picked up straight away.
        if self.local.pending_count().await.unwrap_or(0) > 0 {
            return Duration::ZERO;
        }
        self.set_status(app, SyncStatus::Synced);
        IDLE
    }

    pub fn spawn(self: Arc<Self>, app: AppHandle) {
        tauri::async_runtime::spawn(async move {
            loop {
                let wait = self.round(&app).await;
                // Saves come after every turn; a short pause batches them.
                tokio::select! {
                    _ = self.wake.notified() => tokio::time::sleep(Duration::from_millis(500)).await,
                    _ = tokio::time::sleep(wait) => {}
                }
            }
        });
    }
}
