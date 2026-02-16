-- RegelRecht Upload Portal - Add retention_expiry_date to submissions
-- Migration 003: Retention expiry date

-- Add retention_expiry_date column
ALTER TABLE submissions
ADD COLUMN retention_expiry_date TIMESTAMPTZ;

-- Calculate expiry date for existing submissions (12 months after created_at)
UPDATE submissions
SET retention_expiry_date = created_at + INTERVAL '12 months';

-- Make column NOT NULL after populating existing records
ALTER TABLE submissions
ALTER COLUMN retention_expiry_date SET NOT NULL;

-- Set default for new submissions (12 months from now)
ALTER TABLE submissions
ALTER COLUMN retention_expiry_date SET DEFAULT NOW() + INTERVAL '12 months';

-- Create index for efficient querying of expiring submissions
CREATE INDEX idx_submissions_retention_expiry ON submissions(retention_expiry_date);
