-- Add per-provider enable/disable flag.
ALTER TABLE secrets ADD COLUMN enabled INTEGER NOT NULL DEFAULT 1;
