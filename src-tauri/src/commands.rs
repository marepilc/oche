//! Tauri commands for players, game history and the remote database.

use std::sync::Arc;

use serde::Serialize;
use tauri::State;

use crate::store::local::Imported;
use crate::store::remote::{self, Contents, RemoteConfig};
use crate::store::sync::{Sync, SyncStatus};
use crate::store::{GameRecord, GameSummary, Player};

type Db<'a> = State<'a, Arc<Sync>>;
type Res<T> = Result<T, String>;

fn err(e: sqlx::Error) -> String {
    e.to_string()
}

#[tauri::command]
pub async fn players_list(db: Db<'_>) -> Res<Vec<Player>> {
    db.local.players().await.map_err(err)
}

#[tauri::command]
pub async fn player_named(db: Db<'_>, name: String) -> Res<Player> {
    let name = name.trim();
    if name.is_empty() {
        return Err("empty name".into());
    }
    let player = db.local.player_named(name).await.map_err(err)?;
    db.wake();
    Ok(player)
}

#[tauri::command]
pub async fn game_save(db: Db<'_>, game: GameRecord) -> Res<()> {
    db.local.save_game(&game).await.map_err(err)?;
    db.wake();
    Ok(())
}

#[tauri::command]
pub async fn game_delete(db: Db<'_>, id: String) -> Res<()> {
    db.local.delete_game(&id).await.map_err(err)?;
    db.wake();
    Ok(())
}

#[tauri::command]
pub async fn games_list(db: Db<'_>, limit: Option<i64>) -> Res<Vec<GameSummary>> {
    db.local.games(limit.unwrap_or(100)).await.map_err(err)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteInfo {
    config: RemoteConfig,
    has_password: bool,
}

#[tauri::command]
pub async fn remote_get(db: Db<'_>) -> Res<RemoteInfo> {
    let config = db.config.lock().await.clone();
    let has_password = remote::stored_password(&config).is_some();
    Ok(RemoteInfo { config, has_password })
}

/// A typed password wins; otherwise the one in the keyring is used.
fn password_for(config: &RemoteConfig, typed: Option<String>) -> Option<String> {
    typed.filter(|p| !p.is_empty()).or_else(|| remote::stored_password(config))
}

/// Connects and reports what the database holds, without changing anything.
#[tauri::command]
pub async fn remote_test(config: RemoteConfig, password: Option<String>) -> Res<Contents> {
    let pool = remote::connect(&config, password_for(&config, password).as_deref()).await?;
    let contents = remote::inspect(&pool).await;
    pool.close().await;
    contents
}

/// Creates the schema (or updates it), remembers the settings and pushes the whole local history.
#[tauri::command]
pub async fn remote_connect(db: Db<'_>, mut config: RemoteConfig, password: Option<String>) -> Res<Contents> {
    let typed = password.filter(|p| !p.is_empty());
    let pool = remote::connect(&config, password_for(&config, typed.clone()).as_deref()).await?;
    let contents = match remote::prepare(&pool).await {
        Ok(c) => c,
        Err(e) => {
            pool.close().await;
            return Err(e);
        }
    };
    if let Some(pw) = &typed {
        remote::store_password(&config, pw)?;
    }
    config.enabled = true;
    remote::save_config(&config)?;
    // An existing Oche database may hold games from another computer: take them first.
    if matches!(contents, Contents::Oche { .. }) {
        let (players, games) = remote::fetch_all(&pool).await.map_err(err)?;
        db.local.import(&players, &games).await.map_err(err)?;
    }
    db.local.enqueue_all().await.map_err(err)?;
    db.switch(config, Some(pool)).await;
    Ok(contents)
}

/// Copies players and games that exist only on the remote database.
#[tauri::command]
pub async fn remote_import(db: Db<'_>) -> Res<Imported> {
    let config = db.config.lock().await.clone();
    if !config.enabled {
        return Err("no remote database".into());
    }
    let pool = remote::connect(&config, remote::stored_password(&config).as_deref()).await?;
    let fetched = remote::fetch_all(&pool).await.map_err(err);
    pool.close().await;
    let (players, games) = fetched?;
    db.local.import(&players, &games).await.map_err(err)
}

/// Stops pushing to the remote database. Data already there stays.
#[tauri::command]
pub async fn remote_disconnect(db: Db<'_>) -> Res<()> {
    let mut config = db.config.lock().await.clone();
    remote::forget_password(&config);
    config.enabled = false;
    remote::save_config(&config)?;
    db.local.clear_outbox().await.map_err(err)?;
    db.switch(config, None).await;
    Ok(())
}

#[tauri::command]
pub fn sync_status(db: Db<'_>) -> SyncStatus {
    db.status()
}

#[tauri::command]
pub fn sync_now(db: Db<'_>) {
    db.wake();
}
