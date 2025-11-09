-- =====================================================
-- Migration 7: Multiplayer ChatGPT System
-- Collaborative plan review with AI integration
-- =====================================================

-- Plans table: Core plan storage
CREATE TABLE plans (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title VARCHAR(500) NOT NULL,
    content TEXT NOT NULL,  -- Markdown content
    content_hash VARCHAR(64) NOT NULL,  -- SHA-256 hash for duplicate detection
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    is_public BOOLEAN NOT NULL DEFAULT true,
    current_version INTEGER NOT NULL DEFAULT 1,
    file_size_bytes INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT max_file_size CHECK (file_size_bytes <= 1048576)  -- 1MB limit
);

-- Plan versions: Track evolution of plans
CREATE TABLE plan_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    plan_id UUID NOT NULL REFERENCES plans(id) ON DELETE CASCADE,
    version_number INTEGER NOT NULL,
    content TEXT NOT NULL,
    content_hash VARCHAR(64) NOT NULL,
    change_description TEXT,  -- AI-generated or user-provided description
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(plan_id, version_number)
);

-- Plan comments: Line-based commenting
CREATE TABLE plan_comments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    plan_id UUID NOT NULL REFERENCES plans(id) ON DELETE CASCADE,
    plan_version INTEGER NOT NULL,  -- Which version this comment applies to
    author_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    line_start INTEGER NOT NULL,
    line_end INTEGER NOT NULL,
    comment_text TEXT NOT NULL,
    is_resolved BOOLEAN NOT NULL DEFAULT false,
    resolved_at TIMESTAMPTZ,
    resolved_by UUID REFERENCES users(id) ON DELETE SET NULL,
    resolution_action VARCHAR(20),  -- 'accepted', 'rejected', null
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT valid_line_range CHECK (line_start > 0 AND line_end >= line_start),
    CONSTRAINT valid_resolution_action CHECK (resolution_action IN ('accepted', 'rejected'))
);

-- AI integration jobs: Extended from existing jobs table
-- We'll use the existing jobs table but add AI-specific tracking
CREATE TABLE ai_integration_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_id UUID NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    comment_id UUID NOT NULL REFERENCES plan_comments(id) ON DELETE CASCADE,
    plan_id UUID NOT NULL REFERENCES plans(id) ON DELETE CASCADE,
    prompt_tokens INTEGER,
    completion_tokens INTEGER,
    total_cost_usd DECIMAL(10, 6),
    model_used VARCHAR(100),
    ai_response TEXT,  -- Stores the AI's suggested changes
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- API rate limits: Track usage per user
CREATE TABLE api_rate_limits (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    endpoint VARCHAR(100) NOT NULL,  -- e.g., 'ai_integration'
    request_count INTEGER NOT NULL DEFAULT 0,
    window_start TIMESTAMPTZ NOT NULL,
    window_end TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(user_id, endpoint, window_start)
);

-- Indexes for performance
CREATE INDEX idx_plans_owner ON plans(owner_id);
CREATE INDEX idx_plans_public ON plans(is_public) WHERE is_public = true;
CREATE INDEX idx_plans_created_at ON plans(created_at DESC);
CREATE INDEX idx_plans_content_hash ON plans(content_hash);

CREATE INDEX idx_plan_versions_plan ON plan_versions(plan_id, version_number DESC);
CREATE INDEX idx_plan_versions_created_at ON plan_versions(created_at DESC);

CREATE INDEX idx_plan_comments_plan ON plan_comments(plan_id);
CREATE INDEX idx_plan_comments_author ON plan_comments(author_id);
CREATE INDEX idx_plan_comments_resolved ON plan_comments(is_resolved);
CREATE INDEX idx_plan_comments_version ON plan_comments(plan_id, plan_version);

CREATE INDEX idx_ai_jobs_comment ON ai_integration_jobs(comment_id);
CREATE INDEX idx_ai_jobs_plan ON ai_integration_jobs(plan_id);

CREATE INDEX idx_rate_limits_user ON api_rate_limits(user_id, endpoint);
CREATE INDEX idx_rate_limits_window ON api_rate_limits(window_end);

-- Trigger to update plans.updated_at
CREATE OR REPLACE FUNCTION update_plan_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_plan_timestamp
    BEFORE UPDATE ON plans
    FOR EACH ROW
    EXECUTE FUNCTION update_plan_timestamp();

-- Trigger to update plan_comments.updated_at
CREATE TRIGGER trigger_update_comment_timestamp
    BEFORE UPDATE ON plan_comments
    FOR EACH ROW
    EXECUTE FUNCTION update_plan_timestamp();

-- Cleanup function for old rate limit windows
CREATE OR REPLACE FUNCTION cleanup_old_rate_limits()
RETURNS void AS $$
BEGIN
    DELETE FROM api_rate_limits
    WHERE window_end < NOW() - INTERVAL '7 days';
END;
$$ LANGUAGE plpgsql;

-- Cleanup function for old completed jobs
CREATE OR REPLACE FUNCTION cleanup_old_jobs()
RETURNS void AS $$
BEGIN
    DELETE FROM jobs
    WHERE status IN ('completed', 'failed')
    AND completed_at < NOW() - INTERVAL '30 days';
END;
$$ LANGUAGE plpgsql;

-- Note: To enable automatic cleanup, set up a cron job or pg_cron extension:
-- SELECT cron.schedule('cleanup-rate-limits', '0 0 * * *', 'SELECT cleanup_old_rate_limits()');
-- SELECT cron.schedule('cleanup-old-jobs', '0 2 * * *', 'SELECT cleanup_old_jobs()');
