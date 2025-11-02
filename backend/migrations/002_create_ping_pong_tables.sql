-- Create players table
CREATE TABLE players (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    first_name VARCHAR(255) NOT NULL,
    last_name VARCHAR(255) NOT NULL,
    current_elo FLOAT NOT NULL DEFAULT 1000.0,
    profile_pic BYTEA,  -- Small profile image stored directly (max 15 players, ~100-200KB each)
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Create games table
CREATE TABLE games (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    player1_id UUID NOT NULL REFERENCES players(id),
    player2_id UUID NOT NULL REFERENCES players(id),
    player1_score INT NOT NULL,
    player2_score INT NOT NULL,
    elo_version VARCHAR(50) NOT NULL DEFAULT 'v1',
    played_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Create ELO history table
CREATE TABLE elo_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    player_id UUID NOT NULL REFERENCES players(id),
    game_id UUID NOT NULL REFERENCES games(id),
    elo_before FLOAT NOT NULL,
    elo_after FLOAT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);
