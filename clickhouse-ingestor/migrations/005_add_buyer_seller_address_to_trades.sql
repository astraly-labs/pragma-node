-- +goose Up
-- Add buyer_address and seller_address columns to trades table

ALTER TABLE trades
ADD COLUMN buyer_address String,
ADD COLUMN seller_address String;

-- +goose Down
ALTER TABLE trades
DROP COLUMN buyer_address,
DROP COLUMN seller_address;

