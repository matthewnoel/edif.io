# core Agent Guide

- Shared Rust crate: multiplayer runtime, rule enforcement, adapter interface.
- Not a standalone server; entrypoint lives in `server`.

## Modules
- `adapter.rs`: `GameAdapter` trait.
- `protocol.rs`: websocket message types.
- `game.rs`: room/player state, win rules.
- `server.rs`: axum websocket runtime, room lifecycle.

## Rules
- Server authoritative for prompt outcomes and size updates.
- Room lifecycle: creator is host, match starts on `StartMatch`, lobby-first.
- Win: configurable duration (default 60s), largest player at expiry wins.
- No player consumption; all players stay for full match.
