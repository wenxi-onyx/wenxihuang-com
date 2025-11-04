-- Add dynamic K-factor support to ELO configurations
ALTER TABLE elo_configurations ADD COLUMN IF NOT EXISTS base_k_factor FLOAT;
ALTER TABLE elo_configurations ADD COLUMN IF NOT EXISTS new_player_k_bonus FLOAT;
ALTER TABLE elo_configurations ADD COLUMN IF NOT EXISTS new_player_bonus_period INTEGER;

-- Update existing configurations to use base_k_factor
UPDATE elo_configurations SET
    base_k_factor = k_factor,
    new_player_k_bonus = 0,
    new_player_bonus_period = 0
WHERE base_k_factor IS NULL;

-- Add game count tracking per player (for K-factor decay calculation)
-- Note: We'll calculate this from elo_history records, so no new column needed on players table

-- Create v3 configuration with dynamic K-factor
INSERT INTO elo_configurations (
    version_name,
    k_factor,
    base_k_factor,
    new_player_k_bonus,
    new_player_bonus_period,
    starting_elo,
    description,
    is_active
)
VALUES (
    'v3',
    20.0,  -- This will be the base K, but actual K varies by player experience
    20.0,  -- Base K factor
    48.0,  -- New player K bonus
    10,    -- Games for bonus to decay
    1000.0,
    'Dynamic K-factor: Base K=20, New Player Bonus=48 over 10 games, Starting ELO=1000',
    false
);
