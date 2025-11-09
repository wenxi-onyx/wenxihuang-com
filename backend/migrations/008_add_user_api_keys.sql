-- =====================================================
-- Migration 8: Add User API Keys & Remove Rate Limits
-- Replace server-side rate limiting with user-provided API keys
-- =====================================================

-- Drop the API rate limits table (no longer needed)
DROP TABLE IF EXISTS api_rate_limits CASCADE;

-- Drop the cleanup function for rate limits
DROP FUNCTION IF EXISTS cleanup_old_rate_limits() CASCADE;

-- User API keys: Store encrypted API keys for LLM services
CREATE TABLE IF NOT EXISTS user_api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider VARCHAR(50) NOT NULL,  -- e.g., 'anthropic', 'openai'
    encrypted_key TEXT NOT NULL,  -- Encrypted API key
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(user_id, provider)
);

-- Index for performance
CREATE INDEX IF NOT EXISTS idx_user_api_keys_user ON user_api_keys(user_id, provider);

-- Trigger to update user_api_keys.updated_at
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_trigger
        WHERE tgname = 'trigger_update_api_key_timestamp'
    ) THEN
        CREATE TRIGGER trigger_update_api_key_timestamp
            BEFORE UPDATE ON user_api_keys
            FOR EACH ROW
            EXECUTE FUNCTION update_plan_timestamp();
    END IF;
END $$;
