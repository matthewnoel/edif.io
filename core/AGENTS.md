# core Agent Guide

- Shared Rust crate: multiplayer runtime, rule enforcement, adapter interface.
- Not a standalone server; entrypoint lives in `server`.

## Modules
- `adapter.rs`: `GameAdapter` trait.
- `protocol.rs`: websocket message types.
- `game.rs`: room/player state, win rules.
- `server.rs`: axum websocket runtime, room lifecycle.
- `powerup.rs`: power-up system (see `.cursor/skills/powerups/SKILL.md`).

## Rules
- Server authoritative for prompt outcomes and size updates.
- Room lifecycle: creator is host, match starts on `StartMatch`, lobby-first.
- Win: configurable duration (default 60s), largest player at expiry wins.
- No player consumption; all players stay for full match.
- Prompts are per-player: each player has their own independent prompt (`PlayerState.prompt`). `PromptState` messages are sent to individual players via `send_to_player`, not broadcast. Players answer at their own pace without interrupting each other.

## Testing
- `protocol.rs` `wire_*` tests pin the exact JSON shape of every
  `ServerMessage` / `ClientMessage` variant. Update them (and the
  matching TypeScript fixtures in `client/src/lib/game/protocol.contract.test.ts`)
  in the same change when the wire format intentionally changes.
- `build_app(adapters, config) -> Router` is the public entry point for
  integration tests that need to bind an ephemeral listener; see
  `server/tests/integration.rs`.
