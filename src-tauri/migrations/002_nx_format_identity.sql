-- NEXUS — TWARDY.exe / TW4RDYDEV
-- Durable format identity, migration history and snapshot envelopes.
CREATE TABLE nx_format_metadata(key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE nx_migrations(version INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE, recorded_at TEXT NOT NULL);
ALTER TABLE snapshots ADD COLUMN format_metadata TEXT NOT NULL DEFAULT '{}';
