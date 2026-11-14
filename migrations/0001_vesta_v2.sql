CREATE TABLE app_settings (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);

INSERT INTO app_settings (key, value) VALUES
    ('dashboard_title', 'Server Monitor'),
    ('theme', 'system');

CREATE TABLE nodes (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    endpoint TEXT NOT NULL,
    interval_secs INTEGER NOT NULL CHECK (interval_secs BETWEEN 1 AND 86400),
    position INTEGER NOT NULL
);

CREATE TABLE service_groups (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    columns INTEGER NOT NULL CHECK (columns BETWEEN 1 AND 12),
    position INTEGER NOT NULL
);

CREATE TABLE icons (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    content_type TEXT NOT NULL,
    data BLOB NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE services (
    group_id TEXT NOT NULL REFERENCES service_groups(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    href TEXT,
    icon_id INTEGER REFERENCES icons(id) ON DELETE SET NULL,
    width INTEGER,
    height INTEGER,
    ping_url TEXT,
    widget_name TEXT,
    widget_config_json TEXT CHECK (
        widget_config_json IS NULL OR json_valid(widget_config_json)
    ),
    position INTEGER NOT NULL,
    PRIMARY KEY (group_id, title)
);

CREATE INDEX services_position ON services(group_id, position);

CREATE TABLE admin_account (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    password_hash TEXT NOT NULL
);

CREATE TABLE auth_sessions (
    token_hash TEXT PRIMARY KEY NOT NULL,
    expires_at INTEGER NOT NULL
);
