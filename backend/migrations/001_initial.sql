CREATE TABLE sessions (
    id                  TEXT PRIMARY KEY,
    slug                TEXT NOT NULL,
    node_id             TEXT NOT NULL DEFAULT 'local',
    kind                TEXT NOT NULL,
    state               TEXT NOT NULL,
    cwd                 TEXT NOT NULL,
    env                 TEXT NOT NULL,
    agent_meta          TEXT,
    labels              TEXT NOT NULL,
    created_at          INTEGER NOT NULL,
    last_activity_at    INTEGER NOT NULL,
    exit                TEXT,
    UNIQUE (node_id, slug)
);

CREATE TABLE events (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id  TEXT NOT NULL REFERENCES sessions(id),
    ts          INTEGER NOT NULL,
    kind        TEXT NOT NULL,
    body        BLOB NOT NULL
);
CREATE INDEX events_by_session ON events (session_id, ts);
CREATE INDEX events_by_kind    ON events (kind, ts);

CREATE TABLE metrics_daily (
    day              TEXT PRIMARY KEY,
    tokens_total     INTEGER NOT NULL,
    sessions_created INTEGER NOT NULL,
    push_sent        INTEGER NOT NULL
);
