# Phase 10 — Multi-node scheduler

**Status:** 💭 Aspirational

## Goal

Run hangar backends on multiple machines (optiplex, laptop, cloud VM). Spawn sessions on a chosen node. Migrate sessions live. Single unified dashboard.

## Deliverables (direction)

- Node registration: each backend advertises capacity + hardware
- Session scheduler: pick node by label / budget / manual choice
- Live migration: snapshot session state, transfer bytes, resume on new node
- Control-plane consistency: raft or gossip between backends, or central coordinator
- UI: per-session node indicator, migration action

## Acceptance criteria (draft)

- Register laptop + optiplex as nodes
- Spawn a session explicitly on laptop
- Trigger `POST /sessions/:id/migrate?to=optiplex` and session continues without interruption from the user's perspective
- Dashboard shows all sessions from all nodes in one list

## Open questions

- Coordinator design (decentralized vs one-of-many elected)
- Credential sharing across nodes
- Network topology (all nodes on tailnet assumed)
