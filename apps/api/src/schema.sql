-- wenxihuang.com SQLite schema (port of the original Postgres migrations 001-006).
-- Conventions: UUIDs as TEXT (app-generated), timestamps as TEXT ISO-8601 UTC
-- ("YYYY-MM-DDTHH:MM:SS.SSSZ", lexicographic order == chronological order),
-- booleans as INTEGER 0/1, JSON as TEXT.

-- ===== Authentication =====

CREATE TABLE users (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'user' CHECK (role IN ('admin', 'user')),
    first_name TEXT,
    last_name TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at TEXT NOT NULL,
    last_accessed TEXT NOT NULL
);

CREATE INDEX idx_sessions_user ON sessions(user_id);
CREATE INDEX idx_sessions_expires ON sessions(expires_at);

-- ===== Table tennis core =====

CREATE TABLE players (
    id TEXT PRIMARY KEY,
    first_name TEXT NOT NULL,
    last_name TEXT NOT NULL,
    current_elo REAL NOT NULL DEFAULT 1000.0,
    is_active INTEGER NOT NULL DEFAULT 1,
    profile_pic BLOB,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE elo_configurations (
    id TEXT PRIMARY KEY,
    version_name TEXT NOT NULL UNIQUE,
    k_factor REAL NOT NULL,
    base_k_factor REAL,
    new_player_k_bonus REAL,
    new_player_bonus_period INTEGER,
    starting_elo REAL NOT NULL,
    description TEXT,
    is_active INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    created_by TEXT REFERENCES users(id)
);

CREATE TABLE seasons (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    start_date TEXT NOT NULL UNIQUE,
    starting_elo REAL NOT NULL DEFAULT 1000.0,
    k_factor REAL NOT NULL DEFAULT 32.0,
    base_k_factor REAL,
    new_player_k_bonus REAL,
    new_player_bonus_period INTEGER,
    elo_version TEXT REFERENCES elo_configurations(version_name) ON DELETE SET NULL,
    is_active INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    created_by TEXT REFERENCES users(id)
);

CREATE INDEX idx_seasons_start_date ON seasons(start_date DESC);
CREATE INDEX idx_seasons_active ON seasons(is_active) WHERE is_active = 1;

CREATE TABLE matches (
    id TEXT PRIMARY KEY,
    player1_id TEXT NOT NULL REFERENCES players(id),
    player2_id TEXT NOT NULL REFERENCES players(id),
    season_id TEXT NOT NULL REFERENCES seasons(id),
    submitted_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (player1_id != player2_id)
);

CREATE INDEX idx_matches_player1 ON matches(player1_id);
CREATE INDEX idx_matches_player2 ON matches(player2_id);
CREATE INDEX idx_matches_season ON matches(season_id);
CREATE INDEX idx_matches_submitted_at ON matches(submitted_at DESC);

-- player1_id is ALWAYS the winner of the game, player2_id ALWAYS the loser.
CREATE TABLE games (
    id TEXT PRIMARY KEY,
    match_id TEXT NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    player1_id TEXT NOT NULL REFERENCES players(id),
    player2_id TEXT NOT NULL REFERENCES players(id),
    season_id TEXT NOT NULL REFERENCES seasons(id),
    elo_version TEXT NOT NULL DEFAULT 'v1',
    played_at TEXT NOT NULL,
    CHECK (player1_id != player2_id)
);

CREATE INDEX idx_games_match ON games(match_id);
CREATE INDEX idx_games_season ON games(season_id);
CREATE INDEX idx_games_played_at ON games(played_at DESC);
CREATE INDEX idx_games_player1 ON games(player1_id);
CREATE INDEX idx_games_player2 ON games(player2_id);

CREATE TABLE elo_history (
    id TEXT PRIMARY KEY,
    player_id TEXT NOT NULL REFERENCES players(id),
    game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    elo_before REAL NOT NULL,
    elo_after REAL NOT NULL,
    elo_version TEXT DEFAULT 'v1',
    season_id TEXT NOT NULL REFERENCES seasons(id),
    created_at TEXT NOT NULL
);

CREATE INDEX idx_elo_history_player ON elo_history(player_id);
CREATE INDEX idx_elo_history_game ON elo_history(game_id);
CREATE INDEX idx_elo_history_season ON elo_history(season_id);
CREATE INDEX idx_elo_history_version ON elo_history(elo_version);

CREATE TABLE player_seasons (
    id TEXT PRIMARY KEY,
    player_id TEXT NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    season_id TEXT NOT NULL REFERENCES seasons(id) ON DELETE CASCADE,
    current_elo REAL NOT NULL,
    games_played INTEGER NOT NULL DEFAULT 0,
    wins INTEGER NOT NULL DEFAULT 0,
    losses INTEGER NOT NULL DEFAULT 0,
    is_included INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(player_id, season_id)
);

CREATE INDEX idx_player_seasons_player ON player_seasons(player_id);
CREATE INDEX idx_player_seasons_season ON player_seasons(season_id);
CREATE INDEX idx_player_seasons_elo ON player_seasons(season_id, current_elo DESC);

-- ===== Background jobs =====

CREATE TABLE jobs (
    id TEXT PRIMARY KEY,
    job_type TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending', 'running', 'completed', 'failed')),
    progress INTEGER DEFAULT 0,
    total_items INTEGER,
    processed_items INTEGER DEFAULT 0,
    result_data TEXT,
    created_by TEXT REFERENCES users(id),
    created_at TEXT NOT NULL,
    started_at TEXT,
    completed_at TEXT
);

CREATE INDEX idx_jobs_created_at ON jobs(created_at DESC);
