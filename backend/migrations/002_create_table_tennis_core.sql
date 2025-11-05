-- =====================================================
-- Migration 2: Table Tennis Core System
-- Create players, games, and ELO history tracking
-- =====================================================

-- Create players table
CREATE TABLE players (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    first_name VARCHAR(255) NOT NULL,
    last_name VARCHAR(255) NOT NULL,
    current_elo FLOAT NOT NULL DEFAULT 1000.0,
    is_active BOOLEAN NOT NULL DEFAULT true,
    profile_pic BYTEA,  -- Small profile image stored directly (max 15 players, ~100-200KB each)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create index for filtering active players
CREATE INDEX idx_players_active ON players(is_active) WHERE is_active = true;

-- Create matches table (a match contains multiple games)
CREATE TABLE matches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    player1_id UUID NOT NULL REFERENCES players(id),
    player2_id UUID NOT NULL REFERENCES players(id),
    submitted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT different_players CHECK (player1_id != player2_id)
);

-- Create indexes for matches
CREATE INDEX idx_matches_player1 ON matches(player1_id);
CREATE INDEX idx_matches_player2 ON matches(player2_id);
CREATE INDEX idx_matches_submitted_at ON matches(submitted_at DESC);

-- Create games table (individual games within a match)
-- player1_id is ALWAYS the winner, player2_id is ALWAYS the loser
CREATE TABLE games (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    match_id UUID NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    player1_id UUID NOT NULL REFERENCES players(id),
    player2_id UUID NOT NULL REFERENCES players(id),
    elo_version VARCHAR(50) NOT NULL DEFAULT 'v1',
    played_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT different_players_game CHECK (player1_id != player2_id)
);

-- Create ELO history table
CREATE TABLE elo_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    player_id UUID NOT NULL REFERENCES players(id),
    game_id UUID NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    elo_before FLOAT NOT NULL,
    elo_after FLOAT NOT NULL,
    elo_version VARCHAR(50) DEFAULT 'v1',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create indexes for elo_history
CREATE INDEX idx_elo_history_version ON elo_history(elo_version);

-- Create indexes for games
CREATE INDEX idx_games_match ON games(match_id);
CREATE INDEX idx_games_elo_version ON games(elo_version);
CREATE INDEX idx_games_played_at ON games(played_at DESC);

-- Add trigger to automatically update updated_at on players and matches
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_players_updated_at BEFORE UPDATE ON players
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_matches_updated_at BEFORE UPDATE ON matches
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
