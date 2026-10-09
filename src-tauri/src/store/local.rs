//! The local SQLite database in `~/.local/share/oche/oche.db`.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{SubsecRound, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Row, Sqlite, SqlitePool, Transaction};

use super::{new_id, GameRecord, GameSummary, LegRecord, Player, PlayerResult, TurnRecord};

pub struct Local {
    pub pool: SqlitePool,
    /// Whether changes are queued for the remote database.
    tracking: AtomicBool,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
pub struct Imported {
    pub players: usize,
    pub games: usize,
}

/// A queued change; `id` is the newest outbox row for the entity.
#[derive(Debug, Clone, PartialEq)]
pub struct OutboxItem {
    pub id: i64,
    pub entity: String,
    pub entity_id: String,
    pub op: String,
}

impl Local {
    pub async fn open(path: &Path) -> Result<Self, sqlx::Error> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal);
        Self::with(SqlitePoolOptions::new().max_connections(4).connect_with(options).await?).await
    }

    #[cfg(test)]
    pub async fn memory() -> Result<Self, sqlx::Error> {
        let options = SqliteConnectOptions::new().in_memory(true).foreign_keys(true);
        Self::with(SqlitePoolOptions::new().max_connections(1).connect_with(options).await?).await
    }

    async fn with(pool: SqlitePool) -> Result<Self, sqlx::Error> {
        sqlx::migrate!("./migrations/sqlite").run(&pool).await?;
        Ok(Self { pool, tracking: AtomicBool::new(false) })
    }

    pub fn set_tracking(&self, on: bool) {
        self.tracking.store(on, Ordering::Relaxed);
    }

    async fn enqueue(&self, tx: &mut Transaction<'_, Sqlite>, entity: &str, id: &str, op: &str) -> Result<(), sqlx::Error> {
        if !self.tracking.load(Ordering::Relaxed) {
            return Ok(());
        }
        sqlx::query("INSERT INTO sync_outbox (entity, entity_id, op, created_at) VALUES (?, ?, ?, ?)")
            .bind(entity)
            .bind(id)
            .bind(op)
            .bind(Utc::now())
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    pub async fn players(&self) -> Result<Vec<Player>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, name, created_at, archived_at FROM players WHERE archived_at IS NULL ORDER BY name COLLATE NOCASE",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(player_row).collect()
    }

    pub async fn player(&self, id: &str) -> Result<Option<Player>, sqlx::Error> {
        sqlx::query("SELECT id, name, created_at, archived_at FROM players WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .as_ref()
            .map(player_row)
            .transpose()
    }

    /// The active player with this name (ignoring case), created if there is none.
    pub async fn player_named(&self, name: &str) -> Result<Player, sqlx::Error> {
        let name = name.trim();
        let found = sqlx::query(
            "SELECT id, name, created_at, archived_at FROM players
             WHERE archived_at IS NULL AND name = ? COLLATE NOCASE ORDER BY created_at LIMIT 1",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;
        if let Some(row) = found {
            return player_row(&row);
        }
        let player = Player { id: new_id(), name: name.to_owned(), created_at: Utc::now().trunc_subsecs(6), archived_at: None };
        let mut tx = self.pool.begin().await?;
        sqlx::query("INSERT INTO players (id, name, created_at) VALUES (?, ?, ?)")
            .bind(&player.id)
            .bind(&player.name)
            .bind(player.created_at)
            .execute(&mut *tx)
            .await?;
        self.enqueue(&mut tx, "player", &player.id, "upsert").await?;
        tx.commit().await?;
        Ok(player)
    }

    /// Stores the game, replacing whatever was stored under its id.
    pub async fn save_game(&self, g: &GameRecord) -> Result<(), sqlx::Error> {
        self.write_game(g, true).await
    }

    async fn write_game(&self, g: &GameRecord, track: bool) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO games (id, mode, settings_json, started_at, finished_at, winner_id) VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT (id) DO UPDATE SET mode = excluded.mode, settings_json = excluded.settings_json,
               started_at = excluded.started_at, finished_at = excluded.finished_at, winner_id = excluded.winner_id",
        )
        .bind(&g.id)
        .bind(&g.mode)
        .bind(g.settings.to_string())
        .bind(g.started_at)
        .bind(g.finished_at)
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
            sqlx::query(
                "INSERT INTO legs (id, game_id, set_no, leg_no, starter_id, winner_id) VALUES (?, ?, ?, ?, ?, ?)",
            )
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
        if track {
            self.enqueue(&mut tx, "game", &g.id, "upsert").await?;
        }
        tx.commit().await
    }

    /// Adds players and games from the remote database that are not here yet.
    /// Local data wins: games already stored locally are left alone. Nothing is queued for pushing.
    pub async fn import(&self, players: &[Player], games: &[GameRecord]) -> Result<Imported, sqlx::Error> {
        let mut done = Imported::default();
        for p in players {
            let added = sqlx::query(
                "INSERT INTO players (id, name, created_at, archived_at) VALUES (?, ?, ?, ?) ON CONFLICT (id) DO NOTHING",
            )
            .bind(&p.id)
            .bind(&p.name)
            .bind(p.created_at)
            .bind(p.archived_at)
            .execute(&self.pool)
            .await?;
            done.players += added.rows_affected() as usize;
        }
        for g in games {
            let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM games WHERE id = ?)")
                .bind(&g.id)
                .fetch_one(&self.pool)
                .await?;
            if !exists {
                self.write_game(g, false).await?;
                done.games += 1;
            }
        }
        Ok(done)
    }

    pub async fn delete_game(&self, id: &str) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM games WHERE id = ?").bind(id).execute(&mut *tx).await?;
        self.enqueue(&mut tx, "game", id, "delete").await?;
        tx.commit().await
    }

    pub async fn load_game(&self, id: &str) -> Result<Option<GameRecord>, sqlx::Error> {
        let Some(g) = sqlx::query("SELECT * FROM games WHERE id = ?").bind(id).fetch_optional(&self.pool).await?
        else {
            return Ok(None);
        };
        let players = sqlx::query_scalar("SELECT player_id FROM game_players WHERE game_id = ? ORDER BY seat")
            .bind(id)
            .fetch_all(&self.pool)
            .await?;
        let leg_rows = sqlx::query("SELECT * FROM legs WHERE game_id = ? ORDER BY set_no, leg_no")
            .bind(id)
            .fetch_all(&self.pool)
            .await?;
        let mut legs = Vec::with_capacity(leg_rows.len());
        for l in leg_rows {
            let leg_id: String = l.try_get("id")?;
            let turn_rows = sqlx::query("SELECT * FROM turns WHERE leg_id = ? ORDER BY turn_no")
                .bind(&leg_id)
                .fetch_all(&self.pool)
                .await?;
            let mut turns = Vec::with_capacity(turn_rows.len());
            for t in turn_rows {
                let turn_id: String = t.try_get("id")?;
                let darts = sqlx::query("SELECT segment, ring, points FROM darts WHERE turn_id = ? ORDER BY dart_no")
                    .bind(&turn_id)
                    .fetch_all(&self.pool)
                    .await?
                    .iter()
                    .map(|d| {
                        Ok(super::DartRecord {
                            segment: d.try_get("segment")?,
                            ring: d.try_get("ring")?,
                            points: d.try_get("points")?,
                        })
                    })
                    .collect::<Result<_, sqlx::Error>>()?;
                turns.push(TurnRecord {
                    player_id: t.try_get("player_id")?,
                    turn_no: t.try_get("turn_no")?,
                    target: t.try_get("target")?,
                    score_before: t.try_get("score_before")?,
                    scored: t.try_get("scored")?,
                    bust: t.try_get("bust")?,
                    checkout: t.try_get("checkout")?,
                    darts,
                });
            }
            legs.push(LegRecord {
                set_no: l.try_get("set_no")?,
                leg_no: l.try_get("leg_no")?,
                starter_id: l.try_get("starter_id")?,
                winner_id: l.try_get("winner_id")?,
                turns,
            });
        }
        let settings: String = g.try_get("settings_json")?;
        Ok(Some(GameRecord {
            id: g.try_get("id")?,
            mode: g.try_get("mode")?,
            settings: serde_json::from_str(&settings).unwrap_or_default(),
            started_at: g.try_get("started_at")?,
            finished_at: g.try_get("finished_at")?,
            winner_id: g.try_get("winner_id")?,
            players,
            legs,
        }))
    }

    /// The most recent games, newest first, with per-player totals.
    pub async fn games(&self, limit: i64) -> Result<Vec<GameSummary>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT g.id, g.mode, g.settings_json, g.started_at, g.finished_at, g.winner_id,
               (SELECT COUNT(*) FROM legs l WHERE l.game_id = g.id) AS legs,
               p.id AS player_id, p.name,
               (SELECT COUNT(*) FROM legs l WHERE l.game_id = g.id AND l.winner_id = p.id) AS legs_won,
               (SELECT COALESCE(SUM(t.scored), 0) FROM turns t JOIN legs l ON l.id = t.leg_id
                 WHERE l.game_id = g.id AND t.player_id = p.id) AS scored,
               (SELECT COUNT(*) FROM turns t JOIN legs l ON l.id = t.leg_id
                 WHERE l.game_id = g.id AND t.player_id = p.id) AS turns,
               (SELECT COUNT(*) FROM darts d JOIN turns t ON t.id = d.turn_id JOIN legs l ON l.id = t.leg_id
                 WHERE l.game_id = g.id AND t.player_id = p.id) AS darts
             FROM (SELECT * FROM games ORDER BY started_at DESC LIMIT ?) g
             JOIN game_players gp ON gp.game_id = g.id
             JOIN players p ON p.id = gp.player_id
             ORDER BY g.started_at DESC, g.id, gp.seat",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut games: Vec<GameSummary> = Vec::new();
        for r in rows {
            let id: String = r.try_get("id")?;
            if games.last().is_none_or(|g| g.id != id) {
                let settings: String = r.try_get("settings_json")?;
                games.push(GameSummary {
                    id,
                    mode: r.try_get("mode")?,
                    settings: serde_json::from_str(&settings).unwrap_or_default(),
                    started_at: r.try_get("started_at")?,
                    finished_at: r.try_get("finished_at")?,
                    winner_id: r.try_get("winner_id")?,
                    legs: r.try_get("legs")?,
                    players: Vec::new(),
                });
            }
            games.last_mut().unwrap().players.push(PlayerResult {
                id: r.try_get("player_id")?,
                name: r.try_get("name")?,
                legs_won: r.try_get("legs_won")?,
                scored: r.try_get("scored")?,
                darts: r.try_get("darts")?,
                turns: r.try_get("turns")?,
            });
        }
        Ok(games)
    }

    /// Changes waiting for the remote database, one per entity, oldest first.
    pub async fn pending(&self) -> Result<Vec<OutboxItem>, sqlx::Error> {
        sqlx::query(
            "SELECT o.id, o.entity, o.entity_id, o.op FROM sync_outbox o
             WHERE o.id = (SELECT MAX(id) FROM sync_outbox x WHERE x.entity = o.entity AND x.entity_id = o.entity_id)
             ORDER BY o.id",
        )
        .fetch_all(&self.pool)
        .await?
        .iter()
        .map(|r| {
            Ok(OutboxItem {
                id: r.try_get("id")?,
                entity: r.try_get("entity")?,
                entity_id: r.try_get("entity_id")?,
                op: r.try_get("op")?,
            })
        })
        .collect()
    }

    /// Drops the pushed change and anything older queued for the same entity.
    pub async fn pushed(&self, item: &OutboxItem) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM sync_outbox WHERE entity = ? AND entity_id = ? AND id <= ?")
            .bind(&item.entity)
            .bind(&item.entity_id)
            .bind(item.id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn pending_count(&self) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT COUNT(DISTINCT entity || ':' || entity_id) FROM sync_outbox")
            .fetch_one(&self.pool)
            .await
    }

    /// Queues every player and game, for a freshly connected remote database.
    pub async fn enqueue_all(&self) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let now = Utc::now();
        sqlx::query("DELETE FROM sync_outbox").execute(&mut *tx).await?;
        sqlx::query(
            "INSERT INTO sync_outbox (entity, entity_id, op, created_at)
             SELECT 'player', id, 'upsert', ? FROM players ORDER BY created_at",
        )
        .bind(now)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO sync_outbox (entity, entity_id, op, created_at)
             SELECT 'game', id, 'upsert', ? FROM games ORDER BY started_at",
        )
        .bind(now)
        .execute(&mut *tx)
        .await?;
        tx.commit().await
    }

    pub async fn clear_outbox(&self) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM sync_outbox").execute(&self.pool).await?;
        Ok(())
    }
}

fn player_row(r: &sqlx::sqlite::SqliteRow) -> Result<Player, sqlx::Error> {
    Ok(Player {
        id: r.try_get("id")?,
        name: r.try_get("name")?,
        created_at: r.try_get("created_at")?,
        archived_at: r.try_get("archived_at")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::DartRecord;

    fn dart(segment: i32, ring: &str, points: i32) -> DartRecord {
        DartRecord { segment, ring: ring.into(), points }
    }

    fn game(players: &[&Player], legs: usize) -> GameRecord {
        let ids: Vec<String> = players.iter().map(|p| p.id.clone()).collect();
        GameRecord {
            id: new_id(),
            mode: "x01".into(),
            settings: serde_json::json!({ "start": 501, "doubleOut": true }),
            started_at: Utc::now(),
            finished_at: None,
            winner_id: None,
            players: ids.clone(),
            legs: (0..legs)
                .map(|n| LegRecord {
                    set_no: 1,
                    leg_no: n as i32 + 1,
                    starter_id: Some(ids[0].clone()),
                    winner_id: Some(ids[0].clone()),
                    turns: vec![TurnRecord {
                        player_id: ids[0].clone(),
                        turn_no: 1,
                        target: None,
                        score_before: 501,
                        scored: 140,
                        bust: false,
                        checkout: false,
                        darts: vec![dart(20, "triple", 60), dart(20, "triple", 60), dart(20, "outer", 20)],
                    }],
                })
                .collect(),
        }
    }

    #[tokio::test]
    async fn players_are_found_by_name_ignoring_case() {
        let db = Local::memory().await.unwrap();
        let a = db.player_named("Marek").await.unwrap();
        let b = db.player_named(" marek ").await.unwrap();
        assert_eq!(a.id, b.id);
        assert_eq!(db.players().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn saving_replaces_the_game_and_round_trips() {
        let db = Local::memory().await.unwrap();
        let p = db.player_named("Ala").await.unwrap();
        let mut g = game(&[&p], 1);
        db.save_game(&g).await.unwrap();
        g.legs = game(&[&p], 2).legs;
        g.finished_at = Some(Utc::now());
        g.winner_id = Some(p.id.clone());
        db.save_game(&g).await.unwrap();

        let loaded = db.load_game(&g.id).await.unwrap().unwrap();
        assert_eq!(loaded.legs, g.legs);
        assert_eq!(loaded.winner_id, g.winner_id);
        let turns: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM turns").fetch_one(&db.pool).await.unwrap();
        assert_eq!(turns, 2);
    }

    #[tokio::test]
    async fn summaries_total_per_player() {
        let db = Local::memory().await.unwrap();
        let a = db.player_named("Ala").await.unwrap();
        let b = db.player_named("Bob").await.unwrap();
        db.save_game(&game(&[&a, &b], 2)).await.unwrap();
        let games = db.games(10).await.unwrap();
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].legs, 2);
        let ala = &games[0].players[0];
        assert_eq!((ala.legs_won, ala.scored, ala.darts, ala.turns), (2, 280, 6, 2));
        assert_eq!(games[0].players[1].darts, 0);
    }

    #[tokio::test]
    async fn outbox_collapses_changes_and_only_tracks_when_enabled() {
        let db = Local::memory().await.unwrap();
        let p = db.player_named("Ala").await.unwrap();
        let g = game(&[&p], 1);
        db.save_game(&g).await.unwrap();
        assert!(db.pending().await.unwrap().is_empty());

        db.set_tracking(true);
        db.save_game(&g).await.unwrap();
        db.save_game(&g).await.unwrap();
        let pending = db.pending().await.unwrap();
        assert_eq!(pending.len(), 1);
        db.pushed(&pending[0]).await.unwrap();
        assert_eq!(db.pending_count().await.unwrap(), 0);

        db.enqueue_all().await.unwrap();
        let all = db.pending().await.unwrap();
        assert_eq!(all.iter().map(|i| i.entity.as_str()).collect::<Vec<_>>(), ["player", "game"]);

        db.delete_game(&g.id).await.unwrap();
        assert_eq!(db.pending().await.unwrap().last().unwrap().op, "delete");
    }

    #[tokio::test]
    async fn import_adds_only_what_is_missing_and_queues_nothing() {
        let here = Local::memory().await.unwrap();
        let there = Local::memory().await.unwrap();
        let a = there.player_named("Ala").await.unwrap();
        let g = game(&[&a], 1);
        there.save_game(&g).await.unwrap();

        here.set_tracking(true);
        let first = here.import(&[a.clone()], &[g.clone()]).await.unwrap();
        assert_eq!(first, Imported { players: 1, games: 1 });
        assert_eq!(here.load_game(&g.id).await.unwrap().unwrap().legs, g.legs);
        assert_eq!(here.pending_count().await.unwrap(), 0);
        assert_eq!(here.import(&[a], &[g]).await.unwrap(), Imported::default());
    }
}
