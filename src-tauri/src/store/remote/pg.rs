//! PostgreSQL dialect of the remote database.

use std::str::FromStr;
use std::time::Duration;

use sqlx::postgres::{PgConnectOptions, PgPoolOptions, PgSslMode};
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use super::{Assembler, Contents, RemoteConfig};
use crate::store::{DartRecord, GameRecord, LegRecord, Player, TurnRecord};

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations/postgres");

pub async fn connect(cfg: &RemoteConfig, password: Option<&str>) -> Result<PgPool, sqlx::Error> {
    let ssl = PgSslMode::from_str(&cfg.ssl)?;
    let mut options = PgConnectOptions::new()
        .host(&cfg.host)
        .port(cfg.port)
        .database(&cfg.database)
        .username(&cfg.user)
        .ssl_mode(ssl)
        .application_name("oche");
    if let Some(pw) = password {
        options = options.password(pw);
    }
    PgPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(Duration::from_secs(8))
        .connect_with(options)
        .await
}

pub async fn inspect(pool: &PgPool) -> Result<Contents, sqlx::Error> {
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT table_name::text FROM information_schema.tables WHERE table_schema = current_schema() ORDER BY 1",
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

pub async fn fetch_all(pool: &PgPool) -> Result<(Vec<Player>, Vec<GameRecord>), sqlx::Error> {
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

    let mut a = Assembler::default();
    for r in sqlx::query("SELECT turn_id, segment, ring, points FROM darts ORDER BY turn_id, dart_no").fetch_all(pool).await? {
        let d = DartRecord { segment: r.try_get("segment")?, ring: r.try_get("ring")?, points: r.try_get("points")? };
        a.dart(s(r.try_get("turn_id")?), d);
    }
    for r in sqlx::query("SELECT * FROM turns ORDER BY leg_id, turn_no").fetch_all(pool).await? {
        let t = TurnRecord {
            player_id: s(r.try_get("player_id")?),
            turn_no: r.try_get("turn_no")?,
            target: r.try_get("target")?,
            score_before: r.try_get("score_before")?,
            scored: r.try_get("scored")?,
            bust: r.try_get("bust")?,
            checkout: r.try_get("checkout")?,
            darts: Vec::new(),
        };
        a.turn(&s(r.try_get("id")?), s(r.try_get("leg_id")?), t);
    }
    for r in sqlx::query("SELECT * FROM legs ORDER BY game_id, set_no, leg_no").fetch_all(pool).await? {
        let l = LegRecord {
            set_no: r.try_get("set_no")?,
            leg_no: r.try_get("leg_no")?,
            starter_id: so(r.try_get("starter_id")?),
            winner_id: so(r.try_get("winner_id")?),
            turns: Vec::new(),
        };
        a.leg(&s(r.try_get("id")?), s(r.try_get("game_id")?), l);
    }
    for r in sqlx::query("SELECT game_id, player_id FROM game_players ORDER BY game_id, seat").fetch_all(pool).await? {
        a.seat(s(r.try_get("game_id")?), s(r.try_get("player_id")?));
    }
    let mut games = Vec::new();
    for r in sqlx::query("SELECT * FROM games ORDER BY started_at").fetch_all(pool).await? {
        let settings: sqlx::types::Json<serde_json::Value> = r.try_get("settings_json")?;
        games.push(a.game(GameRecord {
            id: s(r.try_get("id")?),
            mode: r.try_get("mode")?,
            settings: settings.0,
            started_at: r.try_get("started_at")?,
            finished_at: r.try_get("finished_at")?,
            winner_id: so(r.try_get("winner_id")?),
            players: Vec::new(),
            legs: Vec::new(),
        }));
    }
    Ok((players, games))
}
