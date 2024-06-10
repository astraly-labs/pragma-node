-- Your SQL goes here
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE perp_entries (
  id uuid DEFAULT uuid_generate_v4(),
  pair_id VARCHAR NOT NULL,
  price NUMERIC NOT NULL,
  timestamp TIMESTAMPTZ NOT NULL,
  publisher TEXT NOT NULL,
  publisher_signature TEXT NOT NULL,
  source VARCHAR NOT NULL,
  PRIMARY KEY (id, timestamp)
);

CREATE UNIQUE INDEX idx_perp_entries_unique ON perp_entries(pair_id, source, timestamp DESC);

SELECT
  create_hypertable('perp_entries', 'timestamp');