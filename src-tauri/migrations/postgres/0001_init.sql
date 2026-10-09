-- Oche schema, PostgreSQL dialect. Keep in step with migrations/sqlite.

CREATE TABLE players (
    id          UUID PRIMARY KEY,
    name        TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL,
    archived_at TIMESTAMPTZ
);
CREATE INDEX players_name ON players (lower(name));

-- mode: x01 | checkout | scoring | bobs27
CREATE TABLE games (
    id            UUID PRIMARY KEY,
    mode          TEXT NOT NULL,
    settings_json JSONB NOT NULL,
    started_at    TIMESTAMPTZ NOT NULL,
    finished_at   TIMESTAMPTZ,
    winner_id     UUID REFERENCES players (id)
);
CREATE INDEX games_started ON games (started_at);

CREATE TABLE game_players (
    game_id   UUID NOT NULL REFERENCES games (id) ON DELETE CASCADE,
    player_id UUID NOT NULL REFERENCES players (id),
    seat      INTEGER NOT NULL,
    PRIMARY KEY (game_id, seat)
);
CREATE INDEX game_players_player ON game_players (player_id);

-- In training drills a leg is one attempt (checkout) or the whole session.
CREATE TABLE legs (
    id          UUID PRIMARY KEY,
    game_id     UUID NOT NULL REFERENCES games (id) ON DELETE CASCADE,
    set_no      INTEGER NOT NULL,
    leg_no      INTEGER NOT NULL,
    starter_id  UUID REFERENCES players (id),
    winner_id   UUID REFERENCES players (id),
    started_at  TIMESTAMPTZ,
    finished_at TIMESTAMPTZ
);
CREATE INDEX legs_game ON legs (game_id);

-- target: what the turn aimed at in drills (D7, T20, 121), NULL in matches.
CREATE TABLE turns (
    id           UUID PRIMARY KEY,
    leg_id       UUID NOT NULL REFERENCES legs (id) ON DELETE CASCADE,
    player_id    UUID NOT NULL REFERENCES players (id),
    turn_no      INTEGER NOT NULL,
    target       TEXT,
    score_before INTEGER NOT NULL,
    scored       INTEGER NOT NULL,
    bust         BOOLEAN NOT NULL,
    checkout     BOOLEAN NOT NULL
);
CREATE INDEX turns_leg ON turns (leg_id);
CREATE INDEX turns_player ON turns (player_id);

-- ring: inner | outer | single | double | triple | obull | bull | miss
CREATE TABLE darts (
    id         UUID PRIMARY KEY,
    turn_id    UUID NOT NULL REFERENCES turns (id) ON DELETE CASCADE,
    dart_no    INTEGER NOT NULL,
    segment    INTEGER NOT NULL,
    ring       TEXT NOT NULL,
    multiplier INTEGER NOT NULL,
    points     INTEGER NOT NULL
);
CREATE INDEX darts_turn ON darts (turn_id);
