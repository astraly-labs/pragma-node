-- Your SQL goes here
DROP INDEX IF EXISTS idx_entries_publisher_source_timestamp;
CREATE INDEX idx_entries_source ON entries(source);