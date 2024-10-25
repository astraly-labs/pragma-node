-- This file should undo anything in `up.sql`
DELETE FROM public.currencies 
WHERE name IN ('TON', 'JTO', 'OKB', '1000SATS');