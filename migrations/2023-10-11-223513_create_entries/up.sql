-- Your SQL goes here
CREATE TABLE entries (
  id SERIAL PRIMARY KEY,
  pair_id VARCHAR NOT NULL,
  publisher TEXT NOT NULL,
  timestamp TIMESTAMP NOT NULL,
  price NUMERIC NOT NULL
)