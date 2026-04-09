---
name: powerups
description: >-
  Guide for working on the power-up system in edif.io. Use when creating,
  modifying, or debugging power-ups, their timing, effects, or UI.
---

# Power-Ups

## Architecture

Power-ups are server-authoritative. The lifecycle is: **offer -> earn (win a round) -> activate -> expire**.

### Server (Rust)

- `core/src/powerup.rs`: kinds, timing functions, recipient/kind selection, active-effect queries, expiry cleanup.
- `core/src/server.rs`: distribution loop (`start_powerup_timer`), activation on round win in `handle_submission`.
- `core/src/protocol.rs`: `PowerUpOffered`, `PowerUpActivated`, `PowerUpOfferExpired`, `PowerUpEffectEnded` messages.

### Client (Svelte)

- `client/src/lib/game/connection.svelte.ts`: `PendingPowerUp` type, handlers for offer/expire/activate messages.
- `client/src/lib/game/protocol.ts`: `PowerUpKind`, message type guards.
- `client/src/routes/room/[code]/+page.svelte`: ring timers, effect badges, toast, scramble-font class.
- `client/src/lib/components/PowerUpBadge.svelte`: presentational badge.

## Timing

All timing scales with connected player count via `player_scale(n) = (n / 5).clamp(0.4, 1.0)`:

- `distribution_interval(n)`: how often the server considers offering a power-up. Inverse scale (fewer players = longer waits). Clamped to [10s, 25s].
- `offer_duration(n)`: how long a player has to earn an offer by winning a round. Proportional scale.
- `effect_duration(kind, n)`: how long an activated effect lasts. Proportional scale per kind.

The baseline player count is 5 (where current base values apply at 1.0x).

## Adding a New Power-Up Kind

1. Add variant to `PowerUpKind` enum in `core/src/powerup.rs` and `ALL_KINDS` array.
2. Add base duration to `effect_duration` match arm.
3. Implement runtime effect checks (see `is_player_frozen`, `has_double_points` as patterns).
4. Handle activation side-effects in `handle_submission` in `server.rs` if the power-up has an instant effect (like `ScoreSteal`).
5. Add `PowerUpKind` variant string to `client/src/lib/game/protocol.ts` (`isPowerUpKind`).
6. Add metadata entry in `POWERUP_META` in `+page.svelte`.
7. Add any per-effect UI (badges, prompt classes, etc.) to the client.

## Key Invariants

- `ActivePowerUp` stores its `duration` at creation time so snapshots don't need to recompute.
- The distribution loop re-evaluates player count each tick, adapting dynamically to joins/leaves.
- Offers are only given to non-leading players; all tied = no offer.
- The client ring fraction uses `pu.durationMs` from the server, not a hardcoded constant.
