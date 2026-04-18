ALTER TABLE events ADD COLUMN body_text TEXT;

CREATE VIRTUAL TABLE events_fts USING fts5(
    body_text,
    session_id UNINDEXED,
    kind UNINDEXED,
    content='events',
    content_rowid='id',
    tokenize='unicode61'
);
