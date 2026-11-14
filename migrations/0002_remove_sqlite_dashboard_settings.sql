-- Dashboard and app preferences are now owned by config.toml. This migration
-- intentionally removes the former SQLite settings tables after preserving
-- migration 0001 unchanged for databases that already applied it.
DROP TABLE services;
DROP TABLE service_groups;
DROP TABLE nodes;
DROP TABLE app_settings;
