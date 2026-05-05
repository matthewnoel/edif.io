# client Agent Guide

- SvelteKit frontend for all game variants; connects via websocket.
- Client-only blob simulation for visual movement.

## Key Files

- `src/routes/+page.svelte`: lobby.
- `src/routes/room/[code]/+page.svelte`: in-game UI (includes power-up rings/toasts/effects).
- `src/lib/game/connection.svelte.ts`: reactive WebSocket state (includes power-up offer handling).
- `src/lib/game/protocol.ts`: message types (keep aligned with `core/src/protocol.rs`; `protocol.contract.test.ts` pins the JSON shapes on both sides).
- `e2e/home.test.ts`: Playwright smoke tests of the home page; run in CI against the preview build.
- `e2e-fullstack/roundtrip.test.ts`: full-stack Playwright test that boots the Rust server + Vite dev (via `playwright.fullstack.config.ts`) and walks Create Room → Start Match → submit one correct answer.
- `src/lib/game/sim.ts`: clumping/orbit simulation.
- `src/lib/components/PowerUpBadge.svelte`: power-up badge component.

For power-up details see `.cursor/skills/powerups/SKILL.md`.

## Rules

- No gameplay authority on client (sizes, winners, room state are server-driven).
- Keep styles minimal; keep debug panel lightweight.
