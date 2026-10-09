//! MySQL / MariaDB dialect of the remote database.
//! Times are stored as UTC `DATETIME(6)` (a `TIMESTAMP` would end in 2038).

use std::time::Duration;

use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::mysql::{MySqlConnectOptions, MySqlPoolOptions, MySqlSslMode};
use sqlx::{MySql, MySqlPool, Row, Transaction};

use super::{Assembler, Contents, RemoteConfig};
use crate::store::{new_id, DartRecord, GameRecord, LegRecord, Player, TurnRecord};

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations/mysql");

pub async fn connect(cfg: &RemoteConfig, password: Option<&str>) -> Result<MySqlPool, sqlx::Error> {
    let ssl = match cfg.ssl.as_str() {
        "disable" => MySqlSslMode::Disabled,
        "require" => MySqlSslMode::Required,
        _ => MySqlSslMode::Preferred,
    };
    let mut options = MySqlConnectOptions::new()
        .host(&cfg.host)
        .port(cfg.port)
        .database(&cfg.database)
        .username(&cfg.user)
        .ssl_mode(ssl)
        .charset("utf8mb4")
        .timezone(Some(String::from("+00:00")));
    if let Some(pw) = password {
        options = options.password(pw);
    }
    MySqlPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(Duration::from_secs(8))
        .connect_with(options)
        .await
}

pub async fn inspect(pool: &MySqlPool) -> Result<Contents, sqlx::Error> {
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT CAST(table_name AS CHAR) FROM information_schema.tables WHERE table_schema = DATABASE() ORDER BY 1",
    )
    .fetch_all(pool)
    .await?;
    let latest = MIGRATOR.iter().map(|m| m.version).max().unwrap_or(0);
    let version = if tables.iter().any(|t| t == "_sqlx_migrations") {
        sqlx::query_scalar("SELECT MAX(version) FROM _sqlx_migrations WHERE success").fetch_one(pool).await?
    } else {
        None
    };
    Ok(Contents::of(tables, version, latest))
}

fn naive(t: DateTime<Utc>) -> NaiveDateTime {
    t.naive_utc()
}

fn utc(t: NaiveDateTime) -> DateTime<Utc> {
    t.and_utc()
}

async fn upsert_player(tx: &mut Transaction<'_, MySql>, p: &Player) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO players (id, name, created_at, archived_at) VALUES (?, ?, ?, ?)
         ON DUPLICATE KEY UPDATE name = VALUES(name), archived_at = VALUES(archived_at)",
    )
    .bind(&p.id)
    .bind(&p.name)
    .bind(naive(p.created_at))
    .bind(p.archived_at.map(naive))
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn push_player(pool: &MySqlPool, p: &Player) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    upsert_player(&mut tx, p).await?;
    tx.commit().await
}

pub async fn push_game(pool: &MySqlPool, g: &GameRecord, players: &[Player]) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    for p in players {
        upsert_player(&mut tx, p).await?;
    }
    sqlx::query(
        "INSERT INTO games (id, mode, settings_json, started_at, finished_at, winner_id) VALUES (?, ?, ?, ?, ?, ?)
         ON DUPLICATE KEY UPDATE mode = VALUES(mode), settings_json = VALUES(settings_json),
           started_at = VALUES(started_at), finished_at = VALUES(finished_at), winner_id = VALUES(winner_id)",
    )
    .bind(&g.id)
    .bind(&g.mode)
    .bind(g.settings.to_string())
    .bind(naive(g.started_at))
    .bind(g.finished_at.map(naive))
    .bind(&g.winner_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM game_players WHERE game_id = ?").bind(&g.id).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM legs WHERE game_id = ?").bind(&g.id).execute(&mut *tx).await?;

    for (seat, player) in g.players.iter().enumerate() {
        sqlx::query("INSERT INTO game_players (game_id, player_id, seat) VALUES (?, ?, ?)")
            .bind(&g.id)
            .bind(player)
            .bind(seat as i32)
            .execute(&mut *tx)
            .await?;
    }
    for leg in &g.legs {
        let leg_id = new_id();
        sqlx::query("INSERT INTO legs (id, game_id, set_no, leg_no, starter_id, winner_id) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(&leg_id)
            .bind(&g.id)
            .bind(leg.set_no)
            .bind(leg.leg_no)
            .bind(&leg.starter_id)
            .bind(&leg.winner_id)
            .execute(&mut *tx)
            .await?;
        for turn in &leg.turns {
            let turn_id = new_id();
            sqlx::query(
                "INSERT INTO turns (id, leg_id, player_id, turn_no, target, score_before, scored, bust, checkout)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&turn_id)
            .bind(&leg_id)
            .bind(&turn.player_id)
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
                    "INSERT INTO darts (id, turn_id, dart_no, segment, ring, multiplier, points) VALUES (?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(new_id())
                .bind(&turn_id)
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

pub async fn delete_game(pool: &MySqlPool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM games WHERE id = ?").bind(id).execute(pool).await?;
    Ok(())
}

pub async fn fetch_all(pool: &MySqlPool) -> Result<(Vec<Player>, Vec<GameRecord>), sqlx::Error> {
    let players = sqlx::query("SELECT id, name, created_at, archived_at FROM players ORDER BY created_at")
        .fetch_all(pool)
        .await?
        .iter()
        .map(|r| {
            Ok(Player {
                id: r.try_get("id")?,
                name: r.try_get("name")?,
                created_at: utc(r.try_get("created_at")?),
                archived_at: r.try_get::<Option<NaiveDateTime>, _>("archived_at")?.map(utc),
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()?;

    let mut a = Assembler::default();
    for r in sqlx::query("SELECT turn_id, segment, ring, points FROM darts ORDER BY turn_id, dart_no").fetch_all(pool).await? {
        let d = DartRecord { segment: r.try_get("segment")?, ring: r.try_get("ring")?, points: r.try_get("points")? };
        a.dart(r.try_get("turn_id")?, d);
    }
    for r in sqlx::query("SELECT * FROM turns ORDER BY leg_id, turn_no").fetch_all(pool).await? {
        let t = TurnRecord {
            player_id: r.try_get("player_id")?,
            turn_no: r.try_get("turn_no")?,
            target: r.try_get("target")?,
            score_before: r.try_get("score_before")?,
            scored: r.try_get("scored")?,
            bust: r.try_get("bust")?,
            checkout: r.try_get("checkout")?,
            darts: Vec::new(),
        };
        let id: String = r.try_get("id")?;
        a.turn(&id, r.try_get("leg_id")?, t);
    }
    for r in sqlx::query("SELECT * FROM legs ORDER BY game_id, set_no, leg_no").fetch_all(pool).await? {
        let l = LegRecord {
            set_no: r.try_get("set_no")?,
            leg_no: r.try_get("leg_no")?,
            starter_id: r.try_get("starter_id")?,
            winner_id: r.try_get("winner_id")?,
            turns: Vec::new(),
        };
        let id: String = r.try_get("id")?;
        a.leg(&id, r.try_get("game_id")?, l);
    }
    for r in sqlx::query("SELECT game_id, player_id FROM game_players ORDER BY game_id, seat").fetch_all(pool).await? {
        a.seat(r.try_get("game_id")?, r.try_get("player_id")?);
    }
    let mut games = Vec::new();
    for r in sqlx::query("SELECT * FROM games ORDER BY started_at").fetch_all(pool).await? {
        let settings: String = r.try_get("settings_json")?;
        games.push(a.game(GameRecord {
            id: r.try_get("id")?,
            mode: r.try_get("mode")?,
            settings: serde_json::from_str(&settings).unwrap_or_default(),
            started_at: utc(r.try_get("started_at")?),
            finished_at: r.try_get::<Option<NaiveDateTime>, _>("finished_at")?.map(utc),
            winner_id: r.try_get("winner_id")?,
            players: Vec::new(),
            legs: Vec::new(),
        }));
    }
    Ok((players, games))
}
