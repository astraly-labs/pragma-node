-- Your SQL goes here
CREATE INDEX idx_entries_publisher_source_timestamp ON entries(publisher, source, timestamp DESC);