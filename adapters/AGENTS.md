# adapters Agent Guide

- Game-mode crates plugging into `core::GameAdapter`.
- Loaded by `server`; not independent binaries.

## Extension Contract
- Implement `core::GameAdapter` with a stable, unique `game_key()`.
- Keep prompt generation deterministic from seed.
