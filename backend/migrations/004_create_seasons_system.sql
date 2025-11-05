-- =====================================================
-- Migration 4: Seasons System
-- Create seasons for sequential time periods with ELO resets
-- =====================================================

-- Create seasons table for sequential time periods with ELO resets
-- Seasons run sequentially - each season ends when the next one starts
CREATE TABLE seasons (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL UNIQUE,           -- e.g., "Fall 2024", "Spring 2025"
    description TEXT,                             -- Optional description
    start_date TIMESTAMPTZ NOT NULL,              -- Season start date (unique, used for ordering)
    starting_elo FLOAT NOT NULL DEFAULT 1000.0,   -- ELO rating everyone starts with
    k_factor FLOAT NOT NULL DEFAULT 32.0,         -- K-factor for this season
    base_k_factor FLOAT,                          -- Base K-factor for dynamic calculation
    new_player_k_bonus FLOAT,                     -- Bonus K-factor for new players
    new_player_bonus_period INT,                  -- Number of games for bonus period
    is_active BOOLEAN NOT NULL DEFAULT false,     -- The most recent season is active
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID REFERENCES users(id),
    UNIQUE(start_date)                            -- Each season has unique start date
);

-- Index for quick lookup of active season
CREATE INDEX idx_seasons_active ON seasons(is_active) WHERE is_active = true;
CREATE INDEX idx_seasons_start_date ON seasons(start_date DESC);

-- Create player_seasons table to track per-season statistics
CREATE TABLE player_seasons (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    season_id UUID NOT NULL REFERENCES seasons(id) ON DELETE CASCADE,
    current_elo FLOAT NOT NULL,
    games_played INT NOT NULL DEFAULT 0,
    wins INT NOT NULL DEFAULT 0,
    losses INT NOT NULL DEFAULT 0,
    is_included BOOLEAN NOT NULL DEFAULT true,  -- Whether this player is included in this season
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(player_id, season_id)
);

-- Indexes for player_seasons
CREATE INDEX idx_player_seasons_player ON player_seasons(player_id);
CREATE INDEX idx_player_seasons_season ON player_seasons(season_id);
CREATE INDEX idx_player_seasons_elo ON player_seasons(season_id, current_elo DESC);
CREATE INDEX idx_player_seasons_included ON player_seasons(season_id, is_included) WHERE is_included = true;

-- Add comment to explain the column
COMMENT ON COLUMN player_seasons.is_included IS 'Whether this player is included in this season. Excluded players won''t appear in leaderboards or be able to play games in this season.';

-- Add season_id to games table
ALTER TABLE games ADD COLUMN season_id UUID REFERENCES seasons(id);
CREATE INDEX idx_games_season ON games(season_id);

-- Add season_id to elo_history table
ALTER TABLE elo_history ADD COLUMN season_id UUID REFERENCES seasons(id);
CREATE INDEX idx_elo_history_season ON elo_history(season_id);

-- Create default "All-Time" season for existing data
-- This will contain all historical games before seasons were implemented
INSERT INTO seasons (
    name,
    description,
    start_date,
    starting_elo,
    k_factor,
    is_active
) VALUES (
    'All-Time',
    'Historical data before seasons were implemented',
    '2000-01-01 00:00:00+00',  -- Far past date
    1000.0,
    32.0,
    false  -- Not active by default; will activate when first season is created
);

-- Update existing games to belong to All-Time season
UPDATE games SET season_id = (SELECT id FROM seasons WHERE name = 'All-Time')
WHERE season_id IS NULL;

-- Update existing elo_history to belong to All-Time season
UPDATE elo_history SET season_id = (SELECT id FROM seasons WHERE name = 'All-Time')
WHERE season_id IS NULL;

-- Create player_seasons entries for All-Time season
INSERT INTO player_seasons (player_id, season_id, current_elo, games_played, wins, losses)
SELECT
    p.id,
    (SELECT id FROM seasons WHERE name = 'All-Time'),
    p.current_elo,
    COALESCE(
        (SELECT COUNT(*)
         FROM games g
         WHERE (g.player1_id = p.id OR g.player2_id = p.id)),
        0
    ),
    COALESCE(
        (SELECT COUNT(*)
         FROM games g
         WHERE g.player1_id = p.id),
        0
    ),
    COALESCE(
        (SELECT COUNT(*)
         FROM games g
         WHERE g.player2_id = p.id),
        0
    )
FROM players p;

-- Make season_id NOT NULL now that we've backfilled
ALTER TABLE games ALTER COLUMN season_id SET NOT NULL;
ALTER TABLE elo_history ALTER COLUMN season_id SET NOT NULL;

-- Function to automatically update player_seasons.updated_at
CREATE OR REPLACE FUNCTION update_player_seasons_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_player_seasons_updated_at
    BEFORE UPDATE ON player_seasons
    FOR EACH ROW
    EXECUTE FUNCTION update_player_seasons_updated_at();
