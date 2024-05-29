-- Your SQL goes here
CREATE TABLE publishers (
    id uuid DEFAULT uuid_generate_v4(),
    name VARCHAR NOT NULL,
    website_url VARCHAR NOT NULL,
    publisher_type INTEGER NOT NULL CHECK (publisher_type IN (0, 1)), -- 0 = first party, 1 = 3rd party
    master_key VARCHAR NOT NULL,
    active_key VARCHAR NOT NULL,
    active BOOLEAN NOT NULL,
    account_address VARCHAR NOT NULL DEFAULT '',
    PRIMARY KEY (id)
);

INSERT INTO public.publishers (name, website_url, publisher_type, master_key, active_key, active, account_address) 
VALUES
(
    'PRAGMA',
    'https://www.pragma.build/',
    1,
    '0x05e6361b53afbb451d1326ed4e37aecff9ef68af8318eb3c8dc58bcadfc16705',
    '0x05e6361b53afbb451d1326ed4e37aecff9ef68af8318eb3c8dc58bcadfc16705', 
    true, 
    '0x624EBFB99865079BD58CFCFB925B6F5CE940D6F6E41E118B8A72B7163FB435C'
),
(
    'AVNU',
    'https://www.avnu.fi/',
    0,
    '0x0000000000000000000000000000000000000000000000000000000000000042',
    '0x0000000000000000000000000000000000000000000000000000000000000042',
    true, 
    '0x624EBFB99865079BD58CFCFB925FAKEFAKEFAKE6E41E118B8A72B7163FB435C'
);