//! Run against real servers when the variables are set, e.g.
//! `OCHE_TEST_PG=postgres://oche:secret@localhost:5432 OCHE_TEST_MYSQL=mysql://oche:secret@localhost:3306 cargo test remote -- --ignored`.
//! Each server needs an empty database `oche` and a database `other` holding an unrelated table.

use chrono::{SubsecRound, Utc};

use super::*;
use crate::store::local::Local;
use crate::store::{new_id, DartRecord, LegRecord, TurnRecord};

fn config(var: &str, database: &str) -> Option<(RemoteConfig, String)> {
    let url = std::env::var(var).ok()?;
    let (kind, rest) = url.split_once("://")?;
    let (auth, addr) = rest.split_once('@')?;
    let (user, password) = auth.split_once(':')?;
    let (host, port) = addr.trim_end_matches('/').split_once(':')?;
    let cfg = RemoteConfig {
        enabled: true,
        kind: kind.into(),
        host: host.into(),
        port: port.parse().ok()?,
        database: database.into(),
        user: user.into(),
        ssl: "disable".into(),
    };
    Some((cfg, password.into()))
}

async fn empty(remote: &Remote) {
    match remote {
        Remote::Pg(p) => {
            sqlx::query("DROP SCHEMA public CASCADE").execute(p).await.unwrap();
            sqlx::query("CREATE SCHEMA public").execute(p).await.unwrap();
        }
        Remote::MySql(p) => {
            sqlx::query("SET FOREIGN_KEY_CHECKS = 0").execute(p).await.unwrap();
            for t in ["darts", "turns", "legs", "game_players", "games", "players", "_sqlx_migrations"] {
                sqlx::query(sqlx::AssertSqlSafe(format!("DROP TABLE IF EXISTS {t}"))).execute(p).await.unwrap();
            }
            sqlx::query("SET FOREIGN_KEY_CHECKS = 1").execute(p).await.unwrap();
        }
    }
}

async fn creates_schema_and_round_trips(var: &str) {
    let (cfg, pw) = config(var, "oche").expect(var);
    let remote = Remote::connect(&cfg, Some(&pw)).await.unwrap();
    empty(&remote).await;

    assert_eq!(remote.inspect().await.unwrap(), Contents::Empty);
    assert_eq!(remote.prepare().await.unwrap(), Contents::Oche { version: 1, latest: 1 });
    // Preparing again is a no-op.
    assert_eq!(remote.prepare().await.unwrap(), Contents::Oche { version: 1, latest: 1 });

    let local = Local::memory().await.unwrap();
    let p = local.player_named("Zażółć").await.unwrap();
    let mut g = GameRecord {
        id: new_id(),
        mode: "bobs27".into(),
        settings: serde_json::json!({ "kind": "bobs27" }),
        started_at: Utc::now().trunc_subsecs(3),
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
                scored: -2,
                bust: false,
                checkout: true,
                darts: vec![DartRecord { segment: 1, ring: "double".into(), points: 2 }],
            }],
        }],
    };
    remote.push_game(&g, &[p.clone()]).await.unwrap();
    g.finished_at = Some(Utc::now().trunc_subsecs(3));
    let again = g.legs[0].turns[0].clone();
    g.legs[0].turns.push(again);
    remote.push_game(&g, &[p.clone()]).await.unwrap();

    let (players, games) = remote.fetch_all().await.unwrap();
    assert_eq!(players, vec![p.clone()]);
    assert_eq!(games, vec![g.clone()]);

    remote.delete_game(&g.id).await.unwrap();
    assert!(remote.fetch_all().await.unwrap().1.is_empty());
    remote.close().await;
}

async fn refuses_other_tables(var: &str) {
    let (cfg, pw) = config(var, "other").expect(var);
    let remote = Remote::connect(&cfg, Some(&pw)).await.unwrap();
    assert!(matches!(remote.inspect().await.unwrap(), Contents::Foreign { .. }));
    assert!(remote.prepare().await.is_err());
}

async fn rejects_wrong_password(var: &str) {
    let (cfg, _) = config(var, "oche").expect(var);
    assert!(Remote::connect(&cfg, Some("wrong")).await.is_err());
}

#[tokio::test]
#[ignore]
async fn postgres() {
    creates_schema_and_round_trips("OCHE_TEST_PG").await;
    refuses_other_tables("OCHE_TEST_PG").await;
    rejects_wrong_password("OCHE_TEST_PG").await;
}

#[tokio::test]
#[ignore]
async fn mysql() {
    creates_schema_and_round_trips("OCHE_TEST_MYSQL").await;
    refuses_other_tables("OCHE_TEST_MYSQL").await;
    rejects_wrong_password("OCHE_TEST_MYSQL").await;
}
