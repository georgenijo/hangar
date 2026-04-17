-- Rename SessionState values (stored as JSON strings with quotes)
UPDATE sessions SET state = '"booting"'   WHERE state = '"starting"';
UPDATE sessions SET state = '"idle"'      WHERE state = '"running"';
UPDATE sessions SET state = '"exited"'    WHERE state = '"dead"';
UPDATE sessions SET state = '"exited"'    WHERE state = '"exiting"';
UPDATE sessions SET state = '"idle"'      WHERE state = '"paused"';

-- Rewrite SessionKind from bare JSON string to internally-tagged format
UPDATE sessions SET kind = '{"type":"shell"}'        WHERE kind = '"shell"';
UPDATE sessions SET kind = '{"type":"claude_code"}'  WHERE kind = '"claude_code"';
UPDATE sessions SET kind = '{"type":"raw_bytes"}'    WHERE kind = '"raw_bytes"';
