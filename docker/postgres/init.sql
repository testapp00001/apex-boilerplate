-- =============================================================================
-- Apex Database Initialization
-- =============================================================================
-- This script runs when the PostgreSQL container is first created.
-- It sets up extensions and any initial configuration.
-- =============================================================================

-- Enable useful extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Create application schema (optional, for organization)
-- CREATE SCHEMA IF NOT EXISTS apex;

-- Grant privileges (if using separate app user)
-- GRANT ALL PRIVILEGES ON SCHEMA apex TO apex;

-- Log successful initialization
DO $$
BEGIN
    RAISE NOTICE 'Apex database initialized successfully';
END $$;
