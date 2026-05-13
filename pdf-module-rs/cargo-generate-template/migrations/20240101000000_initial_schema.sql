-- Create initial schema for {{project-name}}.
--
-- Run with: sqlx migrate run

CREATE TABLE IF NOT EXISTS users (
    id          TEXT PRIMARY KEY NOT NULL,
    username    TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    roles       TEXT NOT NULL DEFAULT '[]',
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_users_username ON users(username);