-- Uploader Sessions for self-service dossier access
-- Allows uploaders to authenticate using slug + email combination

-- Create uploader sessions table
CREATE TABLE uploader_sessions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    submission_id UUID NOT NULL REFERENCES submissions(id) ON DELETE CASCADE,
    email VARCHAR(255) NOT NULL,
    token_hash VARCHAR(255) NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ip_address VARCHAR(45),
    user_agent VARCHAR(500)
);

-- Index for fast token lookup during session validation
CREATE INDEX idx_uploader_sessions_token ON uploader_sessions(token_hash);

-- Index for finding sessions by submission (useful for cleanup/logout)
CREATE INDEX idx_uploader_sessions_submission ON uploader_sessions(submission_id);

-- Index for cleanup of expired sessions
CREATE INDEX idx_uploader_sessions_expires ON uploader_sessions(expires_at);

-- Add audit actions for uploader login/logout tracking
-- Note: ALTER TYPE with ADD VALUE cannot be inside a transaction in some contexts,
-- but our migration system handles statements individually
ALTER TYPE audit_action ADD VALUE IF NOT EXISTS 'uploader_login';
ALTER TYPE audit_action ADD VALUE IF NOT EXISTS 'uploader_logout';

-- Index for email lookup on submissions table (for login validation)
-- Partial index only covers rows with email set (saves space, faster lookups)
CREATE INDEX idx_submissions_email ON submissions(submitter_email)
WHERE submitter_email IS NOT NULL;
