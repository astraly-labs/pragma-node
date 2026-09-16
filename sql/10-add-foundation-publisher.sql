INSERT INTO publishers (name, website_url, mainnet_address, testnet_address, publisher_type)
SELECT
    'STARKNET_FOUNDATION',
    'https://www.starknet.org/',
    '0x049f7cd6661e0d5df1e27f4636e310b055c6ebbf2d5a87d6514d68b496134903',
    NULL,
    1
WHERE NOT EXISTS (
    SELECT 1 FROM publishers WHERE name = 'STARKNET_FOUNDATION'
);
