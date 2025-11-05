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

-- Create games table
CREATE TABLE games (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    player1_id UUID NOT NULL REFERENCES players(id),
    player2_id UUID NOT NULL REFERENCES players(id),
    player1_score INT NOT NULL,
    player2_score INT NOT NULL,
    elo_version VARCHAR(50) NOT NULL DEFAULT 'v1',
    played_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create ELO history table
CREATE TABLE elo_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    player_id UUID NOT NULL REFERENCES players(id),
    game_id UUID NOT NULL REFERENCES games(id),
    elo_before FLOAT NOT NULL,
    elo_after FLOAT NOT NULL,
    elo_version VARCHAR(50) DEFAULT 'v1',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create indexes for elo_history
CREATE INDEX idx_elo_history_version ON elo_history(elo_version);

-- Create indexes for games
CREATE INDEX idx_games_elo_version ON games(elo_version);

-- Add trigger to automatically update updated_at on players
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_players_updated_at BEFORE UPDATE ON players
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
