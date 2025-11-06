-- Migration: Add Performance Indexes
-- Created: 2025-11-06
-- Description: Adds missing indexes for common query patterns to improve performance

-- Index for elo_history lookups by player
-- Used in get_player_history and get_all_players_history queries
CREATE INDEX IF NOT EXISTS idx_elo_history_player_id ON elo_history(player_id);

-- Index for elo_history lookups by game
-- Used when joining elo_history with games table
CREATE INDEX IF NOT EXISTS idx_elo_history_game_id ON elo_history(game_id);

-- Composite index for player + game lookups (optional but more efficient)
-- Used for queries that filter by both player and game
CREATE INDEX IF NOT EXISTS idx_elo_history_player_game ON elo_history(player_id, game_id);

-- Index for matches lookups by player
-- Used in get_player_matches and match listing queries
CREATE INDEX IF NOT EXISTS idx_matches_player1_id ON matches(player1_id);
CREATE INDEX IF NOT EXISTS idx_matches_player2_id ON matches(player2_id);

-- Index for matches by season (for efficient season-based queries)
CREATE INDEX IF NOT EXISTS idx_matches_season_id ON matches(season_id);

-- Index for matches by submitted_at (for chronological ordering and reassignment)
CREATE INDEX IF NOT EXISTS idx_matches_submitted_at ON matches(submitted_at);

-- Index for games by match (already has foreign key, but explicit index helps)
CREATE INDEX IF NOT EXISTS idx_games_match_id ON games(match_id);

-- Index for games by player (for efficient player game lookups)
CREATE INDEX IF NOT EXISTS idx_games_player1_id ON games(player1_id);
CREATE INDEX IF NOT EXISTS idx_games_player2_id ON games(player2_id);

-- Index for player_seasons lookups
CREATE INDEX IF NOT EXISTS idx_player_seasons_player_id ON player_seasons(player_id);
CREATE INDEX IF NOT EXISTS idx_player_seasons_season_id ON player_seasons(season_id);

-- Composite index for finding active season's players
CREATE INDEX IF NOT EXISTS idx_player_seasons_season_player ON player_seasons(season_id, player_id);

-- Index for seasons by start_date (used in season ordering and reassignment)
CREATE INDEX IF NOT EXISTS idx_seasons_start_date ON seasons(start_date);

-- Index for finding active season quickly
CREATE INDEX IF NOT EXISTS idx_seasons_is_active ON seasons(is_active) WHERE is_active = true;
