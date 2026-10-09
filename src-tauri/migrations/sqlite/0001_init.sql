-- Oche schema, SQLite dialect. Keep in step with migrations/postgres.
-- Ids are UUID v7 strings, timestamps RFC 3339 text in UTC.

CREATE TABLE players (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    archived_at TEXT
);
CREATE INDEX players_name ON players (name COLLATE NOCASE);

-- mode: x01 | checkout | scoring | bobs27
CREATE TABLE games (
    id            TEXT PRIMARY KEY,
    mode          TEXT NOT NULL,
    settings_json TEXT NOT NULL,
    started_at    TEXT NOT NULL,
    finished_at   TEXT,
    winner_id     TEXT REFERENCES players (id)
);
CREATE INDEX games_started ON games (started_at);

CREATE TABLE game_players (
    game_id   TEXT NOT NULL REFERENCES games (id) ON DELETE CASCADE,
    player_id TEXT NOT NULL REFERENCES players (id),
    seat      INTEGER NOT NULL,
    PRIMARY KEY (game_id, seat)
);
CREATE INDEX game_players_player ON game_players (player_id);

-- In training drills a leg is one attempt (checkout) or the whole session.
CREATE TABLE legs (
    id          TEXT PRIMARY KEY,
    game_id     TEXT NOT NULL REFERENCES games (id) ON DELETE CASCADE,
    set_no      INTEGER NOT NULL,
    leg_no      INTEGER NOT NULL,
    starter_id  TEXT REFERENCES players (id),
    winner_id   TEXT REFERENCES players (id),
    started_at  TEXT,
    finished_at TEXT
);
CREATE INDEX legs_game ON legs (game_id);

-- target: what the turn aimed at in drills (D7, T20, 121), NULL in matches.
CREATE TABLE turns (
    id           TEXT PRIMARY KEY,
    leg_id       TEXT NOT NULL REFERENCES legs (id) ON DELETE CASCADE,
    player_id    TEXT NOT NULL REFERENCES players (id),
    turn_no      INTEGER NOT NULL,
    target       TEXT,
    score_before INTEGER NOT NULL,
    scored       INTEGER NOT NULL,
    bust         INTEGER NOT NULL,
    checkout     INTEGER NOT NULL
);
CREATE INDEX turns_leg ON turns (leg_id);
CREATE INDEX turns_player ON turns (player_id);

-- ring: inner | outer | single | double | triple | obull | bull | miss
CREATE TABLE darts (
    id         TEXT PRIMARY KEY,
    turn_id    TEXT NOT NULL REFERENCES turns (id) ON DELETE CASCADE,
    dart_no    INTEGER NOT NULL,
    segment    INTEGER NOT NULL,
    ring       TEXT NOT NULL,
    multiplier INTEGER NOT NULL,
    points     INTEGER NOT NULL
);
CREATE INDEX darts_turn ON darts (turn_id);

-- Changes waiting to be pushed to the remote database. Rows are deleted once pushed;
-- the pusher reads the current local row, so several changes to one game collapse into one push.
CREATE TABLE sync_outbox (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    entity     TEXT NOT NULL,
    entity_id  TEXT NOT NULL,
    op         TEXT NOT NULL,
    created_at TEXT NOT NULL
);
