-- Job tracking table for long-running operations
CREATE TABLE jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_type VARCHAR(50) NOT NULL,  -- e.g., "elo_recalculation"
    status VARCHAR(20) NOT NULL,     -- "pending", "running", "completed", "failed"
    progress INTEGER DEFAULT 0,      -- Percentage 0-100
    total_items INTEGER,             -- Total items to process
    processed_items INTEGER DEFAULT 0, -- Items processed so far
    result_data JSONB,               -- Store result or error information
    created_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ
);

-- Index for quick lookup of running jobs
CREATE INDEX idx_jobs_status ON jobs(status) WHERE status IN ('pending', 'running');
CREATE INDEX idx_jobs_created_at ON jobs(created_at DESC);
