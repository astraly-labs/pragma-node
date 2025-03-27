-- Your SQL goes here
CREATE TABLE publishers (
    id uuid DEFAULT uuid_generate_v4(),
    name VARCHAR NOT NULL,
    master_key VARCHAR NOT NULL,
    active_key VARCHAR NOT NULL,
    account_address VARCHAR NOT NULL DEFAULT '',
    active BOOLEAN NOT NULL DEFAULT true,
    PRIMARY KEY (id)
);

INSERT INTO publishers (name, master_key, active_key, active, account_address) 
VALUES (
    'PRAGMA',
    '0x05e6361b53afbb451d1326ed4e37aecff9ef68af8318eb3c8dc58bcadfc16705',
    '0x05e6361b53afbb451d1326ed4e37aecff9ef68af8318eb3c8dc58bcadfc16705', 
    true, 
    '0x624EBFB99865079BD58CFCFB925B6F5CE940D6F6E41E118B8A72B7163FB435C'
);
