# Phase 3 — Logs firehose

**Status:** ⬜ Planned

## Goal

Add a live log viewer to the dashboard that multiplexes host-level logs (journalctl, systemd units, pane scrollback, app log files) into one filterable stream. Eliminate the need to SSH into the box to debug.

## Non-goals

- Not a full SIEM or log search engine — basic tail + regex filter is the bar
- Not full-text search across history (that's Phase 5's cross-session search)
- No log retention beyond what the source systems already keep

## Deliverables

- Backend: `logs` module with tailers for:
  - `journalctl -f --output=json` (system + all units)
  - Specific systemd unit tails (configurable per-source)
  - Pane scrollback via existing ring-buffer reader
  - Arbitrary log files via `notify::RecommendedWatcher` + tail
- WebSocket `/ws/v1/logs` with per-source subscribe
- Frontend `/logs` page: filter chips, level-colored lines, regex search box, pause/resume, autoscroll toggle
- `config.toml` section for log sources to watch

## Acceptance criteria

- From the phone, open `/logs` and see system + hangar + one pane interleaved in real time
- Filter to a single source and see that source only
- Regex search hides non-matching lines
- Pause freezes scroll; resume catches up without losing lines (buffer in frontend)
- Opening `/logs` doesn't crash on a box with hours of journal history — initial load takes only the last N lines (configurable, default 500)
- Log viewer uses `< 50 MB` RAM on the backend at steady state

## Dependencies

- Phase 2 shipped
- Backend can read `/var/log/journal` (user `george` may need journal group membership)

## Risks / unknowns

- journalctl JSON volume can be huge on a busy host — backend needs to throttle and drop oldest when clients are slow
- Color/ANSI handling across sources

## Estimated effort

2–3 Claude-sessions of work.
