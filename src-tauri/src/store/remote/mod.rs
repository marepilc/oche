//! The optional remote database: PostgreSQL or MySQL/MariaDB.
//!
//! Connection settings live in `~/.config/oche/config.toml` under `[remote]`;
//! the password is kept in the Secret Service keyring, never in the file.

mod mysql;
mod pg;

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sqlx::{MySqlPool, PgPool};

use super::{DartRecord, GameRecord, LegRecord, Player, TurnRecord};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct RemoteConfig {
    pub enabled: bool,
    /// `postgres` or `mysql` (MySQL and MariaDB).
    pub kind: String,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    /// `disable`, `prefer` or `require`.
    pub ssl: String,
}

impl Default for RemoteConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            kind: "postgres".into(),
            host: "localhost".into(),
            port: 5432,
            database: "oche".into(),
            user: "oche".into(),
            ssl: "prefer".into(),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct ConfigFile {
    #[serde(default)]
    remote: RemoteConfig,
    /// Sections owned by other parts of the app are kept as they are.
    #[serde(flatten)]
    rest: toml::Table,
}

fn config_path() -> Option<PathBuf> {
    Some(dirs::config_dir()?.join("oche/config.toml"))
}

fn read_file() -> ConfigFile {
    config_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn load_config() -> RemoteConfig {
    read_file().remote
}

pub fn save_config(cfg: &RemoteConfig) -> Result<(), String> {
    let path = config_path().ok_or("no config directory")?;
    let mut file = read_file();
    file.remote = cfg.clone();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = toml::to_string_pretty(&file).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| e.to_string())
}

fn keyring_entry(cfg: &RemoteConfig) -> Result<keyring::Entry, String> {
    let account = format!("{}://{}@{}:{}/{}", cfg.kind, cfg.user, cfg.host, cfg.port, cfg.database);
    keyring::Entry::new("oche", &account).map_err(|e| format!("keyring: {e}"))
}

pub fn stored_password(cfg: &RemoteConfig) -> Option<String> {
    keyring_entry(cfg).ok()?.get_password().ok()
}

pub fn store_password(cfg: &RemoteConfig, password: &str) -> Result<(), String> {
    keyring_entry(cfg)?.set_password(password).map_err(|e| format!("keyring: {e}"))
}

pub fn forget_password(cfg: &RemoteConfig) {
    if let Ok(entry) = keyring_entry(cfg) {
        let _ = entry.delete_credential();
    }
}

/// What the remote database holds before Oche touches it.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Contents {
    /// No tables: Oche will create its schema.
    Empty,
    /// Already an Oche database; `version` is the newest migration applied.
    Oche { version: i64, latest: i64 },
    /// Holds other tables; Oche refuses to write into it.
    Foreign { tables: Vec<String> },
}

impl Contents {
    /// Classifies a database from its table names; `version` is read only for an Oche database.
    fn of(tables: Vec<String>, version: Option<i64>, latest: i64) -> Self {
        if tables.is_empty() {
            Contents::Empty
        } else if tables.iter().any(|t| t == "_sqlx_migrations") && tables.iter().any(|t| t == "games") {
            Contents::Oche { version: version.unwrap_or(0), latest }
        } else {
            Contents::Foreign { tables }
        }
    }

    fn check_writable(&self) -> Result<(), String> {
        match self {
            Contents::Foreign { tables } => Err(format!(
                "the database is not empty and is not an Oche database (tables: {})",
                tables.join(", ")
            )),
            Contents::Oche { version, latest } if version > latest => {
                Err(format!("the database schema (v{version}) is newer than this version of Oche (v{latest})"))
            }
            _ => Ok(()),
        }
    }
}

/// A connection pool to whichever server is configured.
#[derive(Clone)]
pub enum Remote {
    Pg(PgPool),
    MySql(MySqlPool),
}

impl Remote {
    pub async fn connect(cfg: &RemoteConfig, password: Option<&str>) -> Result<Self, String> {
        let password = password.filter(|p| !p.is_empty());
        let e = |e: sqlx::Error| e.to_string();
        match cfg.kind.as_str() {
            "postgres" => pg::connect(cfg, password).await.map(Remote::Pg).map_err(e),
            "mysql" => mysql::connect(cfg, password).await.map(Remote::MySql).map_err(e),
            other => Err(format!("unknown database kind: {other}")),
        }
    }

    pub async fn close(&self) {
        match self {
            Remote::Pg(p) => p.close().await,
            Remote::MySql(p) => p.close().await,
        }
    }

    pub async fn inspect(&self) -> Result<Contents, String> {
        match self {
            Remote::Pg(p) => pg::inspect(p).await,
            Remote::MySql(p) => mysql::inspect(p).await,
        }
        .map_err(|e| e.to_string())
    }

    /// Creates the schema on an empty database, or brings an Oche database up to date.
    pub async fn prepare(&self) -> Result<Contents, String> {
        self.inspect().await?.check_writable()?;
        match self {
            Remote::Pg(p) => pg::MIGRATOR.run(p).await,
            Remote::MySql(p) => mysql::MIGRATOR.run(p).await,
        }
        .map_err(|e| e.to_string())?;
        self.inspect().await
    }

    pub async fn push_player(&self, p: &Player) -> Result<(), sqlx::Error> {
        match self {
            Remote::Pg(pool) => pg::push_player(pool, p).await,
            Remote::MySql(pool) => mysql::push_player(pool, p).await,
        }
    }

    /// Writes the game with its players, replacing what the remote database had for it.
    pub async fn push_game(&self, g: &GameRecord, players: &[Player]) -> Result<(), sqlx::Error> {
        match self {
            Remote::Pg(pool) => pg::push_game(pool, g, players).await,
            Remote::MySql(pool) => mysql::push_game(pool, g, players).await,
        }
    }

    pub async fn delete_game(&self, id: &str) -> Result<(), sqlx::Error> {
        match self {
            Remote::Pg(pool) => pg::delete_game(pool, id).await,
            Remote::MySql(pool) => mysql::delete_game(pool, id).await,
        }
    }

    /// Everything stored remotely, for importing on another computer.
    pub async fn fetch_all(&self) -> Result<(Vec<Player>, Vec<GameRecord>), sqlx::Error> {
        match self {
            Remote::Pg(pool) => pg::fetch_all(pool).await,
            Remote::MySql(pool) => mysql::fetch_all(pool).await,
        }
    }
}

/// Puts rows read table by table back together into games.
/// Feed darts, turns, legs and seats first (each in order), then the games.
#[derive(Default)]
struct Assembler {
    darts: HashMap<String, Vec<DartRecord>>,
    turns: HashMap<String, Vec<TurnRecord>>,
    legs: HashMap<String, Vec<LegRecord>>,
    seats: HashMap<String, Vec<String>>,
}

impl Assembler {
    fn dart(&mut self, turn_id: String, d: DartRecord) {
        self.darts.entry(turn_id).or_default().push(d);
    }

    fn turn(&mut self, id: &str, leg_id: String, mut t: TurnRecord) {
        t.darts = self.darts.remove(id).unwrap_or_default();
        self.turns.entry(leg_id).or_default().push(t);
    }

    fn leg(&mut self, id: &str, game_id: String, mut l: LegRecord) {
        l.turns = self.turns.remove(id).unwrap_or_default();
        self.legs.entry(game_id).or_default().push(l);
    }

    fn seat(&mut self, game_id: String, player_id: String) {
        self.seats.entry(game_id).or_default().push(player_id);
    }

    fn game(&mut self, mut g: GameRecord) -> GameRecord {
        g.players = self.seats.remove(&g.id).unwrap_or_default();
        g.legs = self.legs.remove(&g.id).unwrap_or_default();
        g
    }
}

#[cfg(test)]
mod tests;
