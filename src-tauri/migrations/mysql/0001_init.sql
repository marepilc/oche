-- Oche schema, MySQL/MariaDB dialect. Keep in step with migrations/sqlite and migrations/postgres.
-- Ids are UUID strings; times are DATETIME(6) in UTC; settings are JSON text.

CREATE TABLE players (
    id          CHAR(36) PRIMARY KEY,
    name        VARCHAR(100) NOT NULL,
    created_at  DATETIME(6) NOT NULL,
    archived_at DATETIME(6) NULL,
    INDEX players_name (name)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4;

-- mode: x01 | cricket | atc | checkout | scoring | bobs27
CREATE TABLE games (
    id            CHAR(36) PRIMARY KEY,
    mode          VARCHAR(20) NOT NULL,
    settings_json LONGTEXT NOT NULL,
    started_at    DATETIME(6) NOT NULL,
    finished_at   DATETIME(6) NULL,
    winner_id     CHAR(36) NULL,
    INDEX games_started (started_at),
    CONSTRAINT games_winner FOREIGN KEY (winner_id) REFERENCES players (id)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4;

CREATE TABLE game_players (
    game_id   CHAR(36) NOT NULL,
    player_id CHAR(36) NOT NULL,
    seat      INT NOT NULL,
    PRIMARY KEY (game_id, seat),
    INDEX game_players_player (player_id),
    CONSTRAINT game_players_game FOREIGN KEY (game_id) REFERENCES games (id) ON DELETE CASCADE,
    CONSTRAINT game_players_p FOREIGN KEY (player_id) REFERENCES players (id)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4;

-- In training drills a leg is one attempt (checkout) or the whole session.
CREATE TABLE legs (
    id          CHAR(36) PRIMARY KEY,
    game_id     CHAR(36) NOT NULL,
    set_no      INT NOT NULL,
    leg_no      INT NOT NULL,
    starter_id  CHAR(36) NULL,
    winner_id   CHAR(36) NULL,
    started_at  DATETIME(6) NULL,
    finished_at DATETIME(6) NULL,
    INDEX legs_game (game_id),
    CONSTRAINT legs_game_fk FOREIGN KEY (game_id) REFERENCES games (id) ON DELETE CASCADE,
    CONSTRAINT legs_starter FOREIGN KEY (starter_id) REFERENCES players (id),
    CONSTRAINT legs_winner FOREIGN KEY (winner_id) REFERENCES players (id)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4;

-- target: what the turn aimed at in drills (D7, T20, 121), NULL in matches.
CREATE TABLE turns (
    id           CHAR(36) PRIMARY KEY,
    leg_id       CHAR(36) NOT NULL,
    player_id    CHAR(36) NOT NULL,
    turn_no      INT NOT NULL,
    target       VARCHAR(20) NULL,
    score_before INT NOT NULL,
    scored       INT NOT NULL,
    bust         BOOLEAN NOT NULL,
    checkout     BOOLEAN NOT NULL,
    INDEX turns_leg (leg_id),
    INDEX turns_player (player_id),
    CONSTRAINT turns_leg_fk FOREIGN KEY (leg_id) REFERENCES legs (id) ON DELETE CASCADE,
    CONSTRAINT turns_player_fk FOREIGN KEY (player_id) REFERENCES players (id)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4;

-- ring: inner | outer | single | double | triple | obull | bull | miss
CREATE TABLE darts (
    id         CHAR(36) PRIMARY KEY,
    turn_id    CHAR(36) NOT NULL,
    dart_no    INT NOT NULL,
    segment    INT NOT NULL,
    ring       VARCHAR(10) NOT NULL,
    multiplier INT NOT NULL,
    points     INT NOT NULL,
    INDEX darts_turn (turn_id),
    CONSTRAINT darts_turn_fk FOREIGN KEY (turn_id) REFERENCES turns (id) ON DELETE CASCADE
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4;
