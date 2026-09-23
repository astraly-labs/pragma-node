INSERT INTO publishers (name, website_url, mainnet_address, testnet_address, publisher_type)
SELECT publisher.name, publisher.website_url, publisher.mainnet_address, NULL, 1
FROM (VALUES
    (
        'STARKWARE',
        'https://starkware.co/',
        '0x05753e99d2fc3132465704a7cc5c2ec8458b17c00aaf3c4deabdc65c29280641'
    ),
    (
        'ARGENT',
        'https://www.ready.co/',
        '0x03235745f167c21fdc2cc2b17f54b55b73d7f2a399cc10df06d5c7f2a5dd6515'
    ),
    (
        'PRAGMA_LP',
        'https://www.pragma.build/',
        '0x0670de32356047cb52a160c34741b1b07517068cc6cd191a09670622b7652a39'
    )
) AS publisher(name, website_url, mainnet_address)
WHERE NOT EXISTS (
    SELECT 1 FROM publishers WHERE name = publisher.name
);
