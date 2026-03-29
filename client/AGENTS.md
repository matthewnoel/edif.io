# client Agent Guide

- SvelteKit frontend for all game variants; connects via websocket.
- Client-only blob simulation for visual movement.

## Key Files

- `src/routes/+page.svelte`: lobby.
- `src/routes/room/[code]/+page.svelte`: in-game UI.
- `src/lib/game/connection.svelte.ts`: reactive WebSocket state.
- `src/lib/game/protocol.ts`: message types (keep aligned with `core/src/protocol.rs`).
- `src/lib/game/sim.ts`: clumping/orbit simulation.

## Rules

- No gameplay authority on client (sizes, winners, room state are server-driven).
- Keep styles minimal; keep debug panel lightweight.
