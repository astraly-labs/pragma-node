-- This file should undo anything in `up.sql`
DROP TABLE IF EXISTS perp_entries;
DROP INDEX IF EXISTS idx_perp_entries_unique;
SELECT detach_table('perp_entries');

