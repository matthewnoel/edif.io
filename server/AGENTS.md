# server Agent Guide

- Thin Rust binary: boots `core` websocket runtime and registers adapters.
- Entry: `src/main.rs`. No gameplay logic here.

## Env Vars
- `BIND_ADDR` (default `0.0.0.0:4000`)
- `GROWTH_PER_ROUND_WIN` (default `4.0`)
- `MATCH_DURATION_SECS` (default `60`)

## Registered Adapters
<!-- GENERATED:REGISTERED_ADAPTERS -->
- `arithmetic` (`edif-io-arithmetic-adapter`): Implements arithmetic prompt racing for edif.io.
- `keyboarding` (`edif-io-keyboarding-adapter`): Implements word-based prompt racing for edif.io.
- `state-abbreviations` (`edif-io-state-abbreviations-adapter`): state-abbreviations adapter.
<!-- /GENERATED:REGISTERED_ADAPTERS -->

## Rules
- Keep this crate thin: orchestration only, not gameplay logic.
- Register adapters explicitly; preserve stable `game_key()` values.
