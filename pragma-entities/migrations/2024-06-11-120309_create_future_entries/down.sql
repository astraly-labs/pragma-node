-- This file should undo anything in `up.sql`
DROP TABLE IF EXISTS future_entries;
DROP INDEX IF EXISTS idx_future_entries_unique;
SELECT detach_table('future_entries');

