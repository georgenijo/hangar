# Phase 11 — Voice + mobile UX

**Status:** 💭 Aspirational

## Goal

First-class phone experience: PWA with push-to-talk, STT, TTS. Widgets and a watch complication for ambient awareness. Voice "hey Claude in wave, summarize recent commits" dispatches a prompt and reads the response back.

## Deliverables (direction)

- PWA install target
- iOS Shortcut bundle for common actions (prompt, broadcast, status)
- Push-to-talk interface with Whisper on box (STT) and local TTS (piper / macOS `say` bridge)
- Widget: last line from each session
- Watch complication: active session count + awaiting count

## Acceptance criteria (draft)

- From the phone's lock screen, raise mic and say a prompt; prompt appears in the chosen session within 5 s
- TTS reads the completed turn aloud
- Widget updates within 30 s of state changes

## Open questions

- Whisper on optiplex's CPU (no GPU) — latency and quality
- Wake word vs tap-to-talk
- Voice privacy — keep audio on box, never leave tailnet
