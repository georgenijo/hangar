# Architectural Decision Records

An ADR is a short doc capturing a non-trivial choice: context, options considered, decision, consequences. We write one when a choice shapes future work or could plausibly be questioned six months later.

## Format

Each file: `NNNN-kebab-case-title.md`. Numbered in order decided. Template:

```markdown
# ADR-NNNN: Title

**Status:** Accepted | Superseded by ADR-NNNN | Deprecated
**Date:** YYYY-MM-DD
**Phase:** Which phase this decision affects

## Context

What problem are we solving? What constraints matter?

## Options considered

1. Option A — trade-offs
2. Option B — trade-offs
3. Option C — trade-offs

## Decision

What we chose.

## Consequences

- Good: …
- Bad: …
- Future work opened up: …
```

## Index

| # | Title | Status |
|---|---|---|
| 0001 | Name the project "hangar" | Accepted |
| 0002 | tmux stays as stopgap, not the substrate | Accepted |
| 0003 | Rust + axum + portable-pty for backend | Accepted |
| 0004 | SQLite + ring-buffer files for persistence | Accepted |
| 0005 | SvelteKit for frontend from day one | Accepted |
| 0006 | No sandboxing in MVP | Accepted |
| 0007 | Session slug is unique per node, not global | Accepted |
| 0008 | No forking/branching in MVP | Accepted |
| 0009 | Labels are key=value maps, not word lists | Accepted |
| 0010 | Sessions survive backend restart via supervisor pattern | Accepted |
| 0011 | Ring-buffer files for raw bytes, SQLite for events | Accepted |
| 0012 | 100 MB output history cap per session | Accepted |
| 0013 | No auth for MVP — tailnet is the boundary | Accepted |
| 0014 | Dashboard visibility is the killer feature | Accepted |
| 0015 | MVP bundles backend + UI + push + REST into one release | Accepted |
