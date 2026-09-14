INSERT INTO publishers (name, website_url, mainnet_address, testnet_address, publisher_type)
SELECT
    'STARKWARE',
    'https://starkware.co/',
    '0x05753e99d2fc3132465704a7cc5c2ec8458b17c00aaf3c4deabdc65c29280641',
    NULL,
    1
WHERE NOT EXISTS (
    SELECT 1 FROM publishers WHERE name = 'STARKWARE'
);
