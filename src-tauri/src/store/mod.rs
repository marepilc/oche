//! Game history: the local SQLite database (always the source of truth)
//! and an optional remote PostgreSQL database that local changes are pushed to.

pub mod local;
pub mod remote;
pub mod sync;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Player {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
}

/// A whole game as the frontend sends it. Saving replaces everything stored for `id`,
/// so the frontend can save after every confirmed turn.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GameRecord {
    pub id: String,
    /// `x01`, `checkout`, `scoring` or `bobs27`.
    pub mode: String,
    pub settings: serde_json::Value,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub winner_id: Option<String>,
    /// Player ids in seat order.
    pub players: Vec<String>,
    pub legs: Vec<LegRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LegRecord {
    pub set_no: i32,
    pub leg_no: i32,
    pub starter_id: Option<String>,
    pub winner_id: Option<String>,
    pub turns: Vec<TurnRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TurnRecord {
    pub player_id: String,
    pub turn_no: i32,
    pub target: Option<String>,
    pub score_before: i32,
    pub scored: i32,
    pub bust: bool,
    pub checkout: bool,
    pub darts: Vec<DartRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DartRecord {
    pub segment: i32,
    pub ring: String,
    pub points: i32,
}

impl DartRecord {
    pub fn multiplier(&self) -> i32 {
        match self.ring.as_str() {
            "double" | "bull" => 2,
            "triple" => 3,
            "miss" => 0,
            _ => 1,
        }
    }
}

/// One row of the history list: a game and how one of its players did.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GameSummary {
    pub id: String,
    pub mode: String,
    pub settings: serde_json::Value,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub winner_id: Option<String>,
    pub legs: i64,
    pub players: Vec<PlayerResult>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlayerResult {
    pub id: String,
    pub name: String,
    pub legs_won: i64,
    pub scored: i64,
    pub darts: i64,
    pub turns: i64,
}

/// Statistics of one player. X01 totals cover every X01 game; `training` lists drill sessions.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlayerStats {
    pub games: i64,
    pub legs_played: i64,
    pub legs_won: i64,
    /// Fewest darts in a won leg.
    pub best_leg: Option<i64>,
    pub darts: i64,
    pub scored: i64,
    pub first9_darts: i64,
    pub first9_scored: i64,
    pub n180: i64,
    pub n140: i64,
    pub n100: i64,
    pub checkouts: i64,
    /// Visits that started on a three-dart finish with double out.
    pub checkout_chances: i64,
    pub best_checkout: i64,
    pub timeline: Vec<GamePoint>,
    /// Darts by board area, in every mode.
    pub heat: Vec<HeatCell>,
    pub training: Vec<GameSummary>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GamePoint {
    pub game_id: String,
    pub started_at: DateTime<Utc>,
    pub average: f64,
    pub first9: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HeatCell {
    pub segment: i32,
    pub ring: String,
    pub count: i64,
}

pub fn new_id() -> String {
    uuid::Uuid::now_v7().to_string()
}
