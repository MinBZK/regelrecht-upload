-- RegelRecht Upload Portal - Initial Database Schema
-- Run with: psql -d regelrecht_upload -f 001_initial.sql

-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- =============================================================================
-- SUBMISSIONS
-- =============================================================================

-- Submission status enum
CREATE TYPE submission_status AS ENUM (
    'draft',           -- Initial state, can still be edited
    'submitted',       -- Submitted for review
    'under_review',    -- Being reviewed by admin
    'approved',        -- Approved for processing
    'rejected',        -- Rejected with feedback
    'forwarded',       -- Forwarded to RegelRecht team
    'completed'        -- Processing completed
);

-- Main submissions table
CREATE TABLE submissions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    slug VARCHAR(50) UNIQUE NOT NULL,
    submitter_name VARCHAR(255) NOT NULL,
    submitter_email VARCHAR(255),
    organization VARCHAR(255) NOT NULL,
    organization_department VARCHAR(255),
    status submission_status NOT NULL DEFAULT 'draft',
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    submitted_at TIMESTAMPTZ,

    CONSTRAINT slug_format CHECK (slug ~ '^[a-z0-9-]+$')
);

CREATE INDEX idx_submissions_slug ON submissions(slug);
CREATE INDEX idx_submissions_status ON submissions(status);
CREATE INDEX idx_submissions_created_at ON submissions(created_at DESC);

-- =============================================================================
-- DOCUMENTS
-- =============================================================================

-- Document category enum
CREATE TYPE document_category AS ENUM (
    'formal_law',           -- Link to wetten.overheid.nl
    'circular',             -- Circulaire
    'implementation_policy', -- Uitvoeringsbeleid
    'work_instruction'      -- Werkinstructies
);

-- Document classification enum
CREATE TYPE document_classification AS ENUM (
    'public',           -- May be published publicly
    'claude_allowed',   -- May be processed by non-EU AI services
    'restricted'        -- Cannot be processed by non-EU AI services
);

-- Documents table
CREATE TABLE documents (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    submission_id UUID NOT NULL REFERENCES submissions(id) ON DELETE CASCADE,
    category document_category NOT NULL,
    classification document_classification NOT NULL,

    -- For formal_law: external link
    external_url VARCHAR(2048),
    external_title VARCHAR(500),

    -- For uploaded files
    filename VARCHAR(255),
    original_filename VARCHAR(255),
    file_path VARCHAR(1024),
    file_size BIGINT,
    mime_type VARCHAR(127),

    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Ensure either external_url (for formal_law) or file_path (for uploads) is set
    CONSTRAINT document_content_check CHECK (
        (category = 'formal_law' AND external_url IS NOT NULL) OR
        (category != 'formal_law' AND file_path IS NOT NULL)
    )
);

CREATE INDEX idx_documents_submission ON documents(submission_id);
CREATE INDEX idx_documents_category ON documents(category);

-- =============================================================================
-- ADMIN USERS
-- =============================================================================

CREATE TABLE admin_users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    username VARCHAR(100) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    display_name VARCHAR(255),
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_login_at TIMESTAMPTZ
);

CREATE INDEX idx_admin_users_username ON admin_users(username);
CREATE INDEX idx_admin_users_email ON admin_users(email);

-- =============================================================================
-- ADMIN SESSIONS
-- =============================================================================

CREATE TABLE admin_sessions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    admin_user_id UUID NOT NULL REFERENCES admin_users(id) ON DELETE CASCADE,
    token_hash VARCHAR(255) NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ip_address VARCHAR(45),
    user_agent VARCHAR(500)
);

CREATE INDEX idx_admin_sessions_token ON admin_sessions(token_hash);
CREATE INDEX idx_admin_sessions_expires ON admin_sessions(expires_at);

-- =============================================================================
-- CALENDAR SLOTS
-- =============================================================================

CREATE TABLE calendar_slots (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    slot_start TIMESTAMPTZ NOT NULL,
    slot_end TIMESTAMPTZ NOT NULL,
    is_available BOOLEAN NOT NULL DEFAULT true,
    booked_by_submission UUID REFERENCES submissions(id) ON DELETE SET NULL,
    created_by UUID REFERENCES admin_users(id),
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT valid_time_range CHECK (slot_end > slot_start)
);

CREATE INDEX idx_calendar_slots_start ON calendar_slots(slot_start);
CREATE INDEX idx_calendar_slots_available ON calendar_slots(is_available, slot_start);

-- =============================================================================
-- AUDIT LOG
-- =============================================================================

CREATE TYPE audit_action AS ENUM (
    'submission_created',
    'submission_updated',
    'submission_submitted',
    'submission_status_changed',
    'document_uploaded',
    'document_deleted',
    'slot_booked',
    'slot_cancelled',
    'admin_login',
    'admin_logout',
    'data_exported',
    'data_deleted'
);

CREATE TABLE audit_log (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    action audit_action NOT NULL,
    entity_type VARCHAR(50) NOT NULL,
    entity_id UUID,
    actor_type VARCHAR(20) NOT NULL, -- 'applicant', 'admin', 'system'
    actor_id UUID,
    actor_ip VARCHAR(45),
    details JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audit_log_entity ON audit_log(entity_type, entity_id);
CREATE INDEX idx_audit_log_created ON audit_log(created_at DESC);
CREATE INDEX idx_audit_log_action ON audit_log(action);

-- =============================================================================
-- RATE LIMITING
-- =============================================================================

CREATE TABLE rate_limit_attempts (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    ip_address VARCHAR(45) NOT NULL,
    endpoint VARCHAR(100) NOT NULL,
    attempted_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_rate_limit_ip_endpoint ON rate_limit_attempts(ip_address, endpoint, attempted_at);

-- Function to clean old rate limit entries
CREATE OR REPLACE FUNCTION cleanup_rate_limits() RETURNS void AS $$
BEGIN
    DELETE FROM rate_limit_attempts WHERE attempted_at < NOW() - INTERVAL '1 hour';
END;
$$ LANGUAGE plpgsql;

-- =============================================================================
-- HELPER FUNCTIONS
-- =============================================================================

-- Auto-update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_submissions_updated_at
    BEFORE UPDATE ON submissions
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Generate unique slug
CREATE OR REPLACE FUNCTION generate_submission_slug() RETURNS VARCHAR(50) AS $$
DECLARE
    new_slug VARCHAR(50);
    slug_exists BOOLEAN;
BEGIN
    LOOP
        -- Generate slug: rr-YYYYMMDD-XXXXX (random 5 chars)
        new_slug := 'rr-' || TO_CHAR(NOW(), 'YYYYMMDD') || '-' ||
                    LOWER(SUBSTRING(MD5(RANDOM()::TEXT) FROM 1 FOR 5));

        SELECT EXISTS(SELECT 1 FROM submissions WHERE slug = new_slug) INTO slug_exists;
        EXIT WHEN NOT slug_exists;
    END LOOP;

    RETURN new_slug;
END;
$$ LANGUAGE plpgsql;
