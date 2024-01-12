-- Your SQL goes here
CREATE TABLE publishers (
    id uuid DEFAULT uuid_generate_v4(),
    name VARCHAR NOT NULL,
    master_key VARCHAR NOT NULL,
    active_key VARCHAR NOT NULL,
    active BOOLEAN NOT NULL,
    PRIMARY KEY (id)
);