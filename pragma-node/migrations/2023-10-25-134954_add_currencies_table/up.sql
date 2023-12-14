-- Your SQL goes here
CREATE TABLE currencies (
    id uuid DEFAULT uuid_generate_v4(),
    name VARCHAR NOT NULL,
    decimals NUMERIC NOT NULL,
    abstract BOOLEAN NOT NULL,
    ethereum_address VARCHAR,
    PRIMARY KEY (id)
);
