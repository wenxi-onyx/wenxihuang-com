-- =====================================================
-- Migration 3: ELO Configuration System
-- Manage different ELO algorithm versions with dynamic K-factor support
-- =====================================================

-- ELO configurations table to store different algorithm versions
CREATE TABLE elo_configurations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    version_name VARCHAR(50) NOT NULL UNIQUE,  -- e.g., "v1", "v2", "aggressive"
    k_factor FLOAT NOT NULL,                    -- ELO K-factor (typically 16-40)
    base_k_factor FLOAT,                        -- Base K-factor for dynamic calculation
    new_player_k_bonus FLOAT,                   -- Bonus K-factor for new players
    new_player_bonus_period INTEGER,            -- Number of games for bonus to decay
    starting_elo FLOAT NOT NULL,                -- Initial rating (typically 1000-1500)
    description TEXT,                           -- User-friendly description
    is_active BOOLEAN NOT NULL DEFAULT false,   -- Which version is currently active
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID REFERENCES users(id)        -- Admin who created this config
);

-- Index for quick lookup of active configuration
CREATE INDEX idx_elo_configurations_active ON elo_configurations(is_active) WHERE is_active = true;

-- Default v1 configuration (standard ELO)
INSERT INTO elo_configurations (
    version_name,
    k_factor,
    base_k_factor,
    new_player_k_bonus,
    new_player_bonus_period,
    starting_elo,
    description,
    is_active
) VALUES (
    'v1',
    32.0,
    32.0,
    0,
    0,
    1000.0,
    'Standard ELO with K=32, starting at 1000',
    true
);

-- v2 configuration with dynamic K-factor
INSERT INTO elo_configurations (
    version_name,
    k_factor,
    base_k_factor,
    new_player_k_bonus,
    new_player_bonus_period,
    starting_elo,
    description,
    is_active
) VALUES (
    'v2',
    20.0,
    20.0,
    48.0,
    10,
    1000.0,
    'Dynamic K-factor: Base K=20, New Player Bonus=48 over 10 games, Starting ELO=1000',
    false
);
