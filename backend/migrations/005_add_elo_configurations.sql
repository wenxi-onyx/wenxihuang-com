-- ELO configurations table to store different algorithm versions
CREATE TABLE elo_configurations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    version_name VARCHAR(50) NOT NULL UNIQUE,  -- e.g., "v1", "v2", "aggressive"
    k_factor FLOAT NOT NULL,                    -- ELO K-factor (typically 16-40)
    starting_elo FLOAT NOT NULL,                -- Initial rating (typically 1000-1500)
    description TEXT,                           -- User-friendly description
    is_active BOOLEAN NOT NULL DEFAULT false,   -- Which version is currently active
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID REFERENCES users(id)        -- Admin who created this config
);

-- Default configuration
INSERT INTO elo_configurations (version_name, k_factor, starting_elo, description, is_active)
VALUES ('v1', 32.0, 1000.0, 'Standard ELO with K=32, starting at 1000', true);

-- Index for quick lookup of active configuration
CREATE INDEX idx_elo_configurations_active ON elo_configurations(is_active) WHERE is_active = true;

-- Modify elo_history to reference the configuration used
ALTER TABLE elo_history ADD COLUMN IF NOT EXISTS elo_version VARCHAR(50) DEFAULT 'v1';
CREATE INDEX IF NOT EXISTS idx_elo_history_version ON elo_history(elo_version);

-- Add version to games table
ALTER TABLE games ADD COLUMN IF NOT EXISTS elo_version VARCHAR(50) DEFAULT 'v1';
CREATE INDEX IF NOT EXISTS idx_games_elo_version ON games(elo_version);
