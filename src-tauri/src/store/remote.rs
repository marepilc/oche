//! The optional remote PostgreSQL database.
//!
//! Connection settings live in `~/.config/oche/config.toml` under `[remote]`;
//! the password is kept in the Secret Service keyring, never in the file.

use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions, PgSslMode};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::{DartRecord, GameRecord, LegRecord, Player, TurnRecord};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct RemoteConfig {
    pub enabled: bool,
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
    let account = format!("postgres://{}@{}:{}/{}", cfg.user, cfg.host, cfg.port, cfg.database);
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

pub async fn connect(cfg: &RemoteConfig, password: Option<&str>) -> Result<PgPool, String> {
    let ssl = PgSslMode::from_str(&cfg.ssl).map_err(|e| e.to_string())?;
    let mut options = PgConnectOptions::new()
        .host(&cfg.host)
        .port(cfg.port)
        .database(&cfg.database)
        .username(&cfg.user)
        .ssl_mode(ssl)
        .application_name("oche");
    if let Some(pw) = password.filter(|p| !p.is_empty()) {
        options = options.password(pw);
    }
    PgPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(Duration::from_secs(8))
        .connect_with(options)
        .await
        .map_err(|e| e.to_string())
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

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations/postgres");

pub async fn inspect(pool: &PgPool) -> Result<Contents, String> {
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT table_name::text FROM information_schema.tables WHERE table_schema = current_schema() ORDER BY 1",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    let latest = MIGRATOR.iter().map(|m| m.version).max().unwrap_or(0);
    if tables.is_empty() {
        return Ok(Contents::Empty);
    }
    if tables.iter().any(|t| t == "_sqlx_migrations") && tables.iter().any(|t| t == "games") {
        let version: Option<i64> = sqlx::query_scalar("SELECT MAX(version) FROM _sqlx_migrations WHERE success")
            .fetch_one(pool)
            .await
            .map_err(|e| e.to_string())?;
        return Ok(Contents::Oche { version: version.unwrap_or(0), latest });
    }
    Ok(Contents::Foreign { tables })
}

/// Creates the schema on an empty database, or brings an Oche database up to date.
pub async fn prepare(pool: &PgPool) -> Result<Contents, String> {
    match inspect(pool).await? {
        Contents::Foreign { tables } => Err(format!(
            "the database is not empty and is not an Oche database (tables: {})",
            tables.join(", ")
        )),
        Contents::Oche { version, latest } if version > latest => {
            Err(format!("the database schema (v{version}) is newer than this version of Oche (v{latest})"))
        }
        _ => {
            MIGRATOR.run(pool).await.map_err(|e| e.to_string())?;
            inspect(pool).await
        }
    }
}

fn uuid(s: &str) -> Result<Uuid, sqlx::Error> {
    Uuid::parse_str(s).map_err(|e| sqlx::Error::Decode(Box::new(e)))
}

fn uuid_opt(s: &Option<String>) -> Result<Option<Uuid>, sqlx::Error> {
    s.as_deref().map(uuid).transpose()
}

async fn upsert_player(tx: &mut Transaction<'_, Postgres>, p: &Player) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO players (id, name, created_at, archived_at) VALUES ($1, $2, $3, $4)
         ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, archived_at = EXCLUDED.archived_at",
    )
    .bind(uuid(&p.id)?)
    .bind(&p.name)
    .bind(p.created_at)
    .bind(p.archived_at)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn push_player(pool: &PgPool, p: &Player) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    upsert_player(&mut tx, p).await?;
    tx.commit().await
}

/// Writes the game with its players, replacing what the remote database had for it.
pub async fn push_game(pool: &PgPool, g: &GameRecord, players: &[Player]) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    for p in players {
        upsert_player(&mut tx, p).await?;
    }
    let id = uuid(&g.id)?;
    sqlx::query(
        "INSERT INTO games (id, mode, settings_json, started_at, finished_at, winner_id) VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (id) DO UPDATE SET mode = EXCLUDED.mode, settings_json = EXCLUDED.settings_json,
           started_at = EXCLUDED.started_at, finished_at = EXCLUDED.finished_at, winner_id = EXCLUDED.winner_id",
    )
    .bind(id)
    .bind(&g.mode)
    .bind(sqlx::types::Json(&g.settings))
    .bind(g.started_at)
    .bind(g.finished_at)
    .bind(uuid_opt(&g.winner_id)?)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM game_players WHERE game_id = $1").bind(id).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM legs WHERE game_id = $1").bind(id).execute(&mut *tx).await?;

    for (seat, player) in g.players.iter().enumerate() {
        sqlx::query("INSERT INTO game_players (game_id, player_id, seat) VALUES ($1, $2, $3)")
            .bind(id)
            .bind(uuid(player)?)
            .bind(seat as i32)
            .execute(&mut *tx)
            .await?;
    }
    for leg in &g.legs {
        let leg_id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO legs (id, game_id, set_no, leg_no, starter_id, winner_id) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(leg_id)
        .bind(id)
        .bind(leg.set_no)
        .bind(leg.leg_no)
        .bind(uuid_opt(&leg.starter_id)?)
        .bind(uuid_opt(&leg.winner_id)?)
        .execute(&mut *tx)
        .await?;
        for turn in &leg.turns {
            let turn_id = Uuid::now_v7();
            sqlx::query(
                "INSERT INTO turns (id, leg_id, player_id, turn_no, target, score_before, scored, bust, checkout)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            )
            .bind(turn_id)
            .bind(leg_id)
            .bind(uuid(&turn.player_id)?)
            .bind(turn.turn_no)
            .bind(&turn.target)
            .bind(turn.score_before)
            .bind(turn.scored)
            .bind(turn.bust)
            .bind(turn.checkout)
            .execute(&mut *tx)
            .await?;
            for (n, d) in turn.darts.iter().enumerate() {
                sqlx::query(
                    "INSERT INTO darts (id, turn_id, dart_no, segment, ring, multiplier, points)
                     VALUES ($1, $2, $3, $4, $5, $6, $7)",
                )
                .bind(Uuid::now_v7())
                .bind(turn_id)
                .bind(n as i32 + 1)
                .bind(d.segment)
                .bind(&d.ring)
                .bind(d.multiplier())
                .bind(d.points)
                .execute(&mut *tx)
                .await?;
            }
        }
    }
    tx.commit().await
}

pub async fn delete_game(pool: &PgPool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM games WHERE id = $1").bind(uuid(id)?).execute(pool).await?;
    Ok(())
}

/// Everything stored remotely, for importing on another computer.
pub async fn fetch_all(pool: &PgPool) -> Result<(Vec<Player>, Vec<GameRecord>), sqlx::Error> {
    use std::collections::HashMap;
    use sqlx::Row;

    let s = |u: Uuid| u.to_string();
    let so = |u: Option<Uuid>| u.map(|u| u.to_string());

    let players = sqlx::query("SELECT id, name, created_at, archived_at FROM players ORDER BY created_at")
        .fetch_all(pool)
        .await?
        .iter()
        .map(|r| {
            Ok(Player {
                id: s(r.try_get("id")?),
                name: r.try_get("name")?,
                created_at: r.try_get("created_at")?,
                archived_at: r.try_get("archived_at")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()?;

    let mut darts: HashMap<Uuid, Vec<DartRecord>> = HashMap::new();
    for r in sqlx::query("SELECT turn_id, segment, ring, points FROM darts ORDER BY turn_id, dart_no").fetch_all(pool).await? {
        darts.entry(r.try_get("turn_id")?).or_default().push(DartRecord {
            segment: r.try_get("segment")?,
            ring: r.try_get("ring")?,
            points: r.try_get("points")?,
        });
    }
    let mut turns: HashMap<Uuid, Vec<TurnRecord>> = HashMap::new();
    for r in sqlx::query("SELECT * FROM turns ORDER BY leg_id, turn_no").fetch_all(pool).await? {
        let id: Uuid = r.try_get("id")?;
        turns.entry(r.try_get("leg_id")?).or_default().push(TurnRecord {
            player_id: s(r.try_get("player_id")?),
            turn_no: r.try_get("turn_no")?,
            target: r.try_get("target")?,
            score_before: r.try_get("score_before")?,
            scored: r.try_get("scored")?,
            bust: r.try_get("bust")?,
            checkout: r.try_get("checkout")?,
            darts: darts.remove(&id).unwrap_or_default(),
        });
    }
    let mut legs: HashMap<Uuid, Vec<LegRecord>> = HashMap::new();
    for r in sqlx::query("SELECT * FROM legs ORDER BY game_id, set_no, leg_no").fetch_all(pool).await? {
        let id: Uuid = r.try_get("id")?;
        legs.entry(r.try_get("game_id")?).or_default().push(LegRecord {
            set_no: r.try_get("set_no")?,
            leg_no: r.try_get("leg_no")?,
            starter_id: so(r.try_get("starter_id")?),
            winner_id: so(r.try_get("winner_id")?),
            turns: turns.remove(&id).unwrap_or_default(),
        });
    }
    let mut seats: HashMap<Uuid, Vec<String>> = HashMap::new();
    for r in sqlx::query("SELECT game_id, player_id FROM game_players ORDER BY game_id, seat").fetch_all(pool).await? {
        seats.entry(r.try_get("game_id")?).or_default().push(s(r.try_get("player_id")?));
    }
    let mut games = Vec::new();
    for r in sqlx::query("SELECT * FROM games ORDER BY started_at").fetch_all(pool).await? {
        let id: Uuid = r.try_get("id")?;
        let settings: sqlx::types::Json<serde_json::Value> = r.try_get("settings_json")?;
        games.push(GameRecord {
            id: s(id),
            mode: r.try_get("mode")?,
            settings: settings.0,
            started_at: r.try_get("started_at")?,
            finished_at: r.try_get("finished_at")?,
            winner_id: so(r.try_get("winner_id")?),
            players: seats.remove(&id).unwrap_or_default(),
            legs: legs.remove(&id).unwrap_or_default(),
        });
    }
    Ok((players, games))
}

/// Runs against a real server when `OCHE_TEST_PG` is set, e.g.
/// `OCHE_TEST_PG=postgres://oche:secret@localhost:5432 cargo test remote -- --ignored`.
/// It needs an empty database `oche` and a database `other` holding an unrelated table.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::local::Local;
    use chrono::Utc;

    fn config(database: &str) -> Option<(RemoteConfig, String)> {
        let url = std::env::var("OCHE_TEST_PG").ok()?;
        let rest = url.strip_prefix("postgres://")?;
        let (auth, addr) = rest.split_once('@')?;
        let (user, password) = auth.split_once(':')?;
        let (host, port) = addr.trim_end_matches('/').split_once(':')?;
        let cfg = RemoteConfig {
            enabled: true,
            host: host.into(),
            port: port.parse().ok()?,
            database: database.into(),
            user: user.into(),
            ssl: "disable".into(),
        };
        Some((cfg, password.into()))
    }

    #[tokio::test]
    #[ignore]
    async fn creates_schema_on_empty_database_and_pushes_games() {
        let (cfg, pw) = config("oche").expect("OCHE_TEST_PG");
        let pool = connect(&cfg, Some(&pw)).await.unwrap();
        sqlx::query("DROP SCHEMA public CASCADE").execute(&pool).await.unwrap();
        sqlx::query("CREATE SCHEMA public").execute(&pool).await.unwrap();

        assert_eq!(inspect(&pool).await.unwrap(), Contents::Empty);
        assert_eq!(prepare(&pool).await.unwrap(), Contents::Oche { version: 1, latest: 1 });
        // Preparing again is a no-op.
        assert_eq!(prepare(&pool).await.unwrap(), Contents::Oche { version: 1, latest: 1 });

        let local = Local::memory().await.unwrap();
        let p = local.player_named("Ala").await.unwrap();
        let mut g = GameRecord {
            id: crate::store::new_id(),
            mode: "bobs27".into(),
            settings: serde_json::json!({ "kind": "bobs27" }),
            started_at: Utc::now(),
            finished_at: None,
            winner_id: None,
            players: vec![p.id.clone()],
            legs: vec![LegRecord {
                set_no: 1,
                leg_no: 1,
                starter_id: Some(p.id.clone()),
                winner_id: None,
                turns: vec![TurnRecord {
                    player_id: p.id.clone(),
                    turn_no: 1,
                    target: Some("D1".into()),
                    score_before: 27,
                    scored: 2,
                    bust: false,
                    checkout: false,
                    darts: vec![DartRecord { segment: 1, ring: "double".into(), points: 2 }],
                }],
            }],
        };
        push_game(&pool, &g, &[p.clone()]).await.unwrap();
        g.finished_at = Some(Utc::now());
        let again = g.legs[0].turns[0].clone();
        g.legs[0].turns.push(again);
        push_game(&pool, &g, &[p.clone()]).await.unwrap();

        let darts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM darts").fetch_one(&pool).await.unwrap();
        assert_eq!(darts, 2);
        let finished: bool = sqlx::query_scalar("SELECT finished_at IS NOT NULL FROM games").fetch_one(&pool).await.unwrap();
        assert!(finished);

        let (players, games) = fetch_all(&pool).await.unwrap();
        assert_eq!(players, vec![p.clone()]);
        assert_eq!(games.len(), 1);
        assert_eq!((&games[0].id, &games[0].players, &games[0].legs), (&g.id, &g.players, &g.legs));

        delete_game(&pool, &g.id).await.unwrap();
        let turns: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM turns").fetch_one(&pool).await.unwrap();
        assert_eq!(turns, 0);
    }

    #[tokio::test]
    #[ignore]
    async fn refuses_a_database_with_other_tables() {
        let (cfg, pw) = config("other").expect("OCHE_TEST_PG");
        let pool = connect(&cfg, Some(&pw)).await.unwrap();
        assert!(matches!(inspect(&pool).await.unwrap(), Contents::Foreign { .. }));
        assert!(prepare(&pool).await.is_err());
    }

    #[tokio::test]
    #[ignore]
    async fn reports_a_wrong_password() {
        let (cfg, _) = config("oche").expect("OCHE_TEST_PG");
        assert!(connect(&cfg, Some("wrong")).await.is_err());
    }
}
