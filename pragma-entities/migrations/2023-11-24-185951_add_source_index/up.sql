-- Your SQL goes here
CREATE UNIQUE INDEX idx_entries_unique
  ON entries(pair_id, source, timestamp);