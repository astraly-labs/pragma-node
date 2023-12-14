-- Your SQL goes here
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE entries (
  id uuid DEFAULT uuid_generate_v4(),
  pair_id VARCHAR NOT NULL,
  publisher TEXT NOT NULL,
  timestamp TIMESTAMP NOT NULL,
  price NUMERIC NOT NULL,
  PRIMARY KEY (id)
)