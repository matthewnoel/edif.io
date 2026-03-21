# edif.io

edif.io is a multiplayer prompt-race game with:

- a single SvelteKit client (`client`)
- shared Rust core runtime (`core`)
- pluggable game adapters (`adapters/*`)
- one game server binary (`server`)

Players choose a game mode on the pregame screen when creating a room. Room game mode is authoritative for all players in that room.

## Prerequisites

### Rust

[Installation instructions](https://rust-lang.org/tools/install/)

### Node.js version `./client/.nvmrc`

Recommended Node Version Managers: [fnm](https://github.com/Schniz/fnm) or [nvm](https://github.com/nvm-sh/nvm)

## Run Locally

### 1. Start the unified game server

From repo root:

```sh
cargo run -p server
```

By default it binds to `0.0.0.0:4000`.

Optional server env vars:

- `BIND_ADDR` (default: `0.0.0.0:4000`)
- `GROWTH_PER_ROUND_WIN` (default: `4.0`)

Example:

```sh
BIND_ADDR=0.0.0.0:4000 GROWTH_PER_ROUND_WIN=4 cargo run -p server
```

### 2. Start the client

In a second terminal:

```sh
cd client
nvm use
npm install
npm run dev
```

Client defaults to connecting to `/ws` on the same host, so with local defaults it will connect to the server on port `4000`.

## Test and Validation

From repo root:

```sh
cargo test --workspace
```

From `client`:

```sh
npm run check
npm run lint
npm run test:unit -- --run
```

## Adding a New Game Mode

1. Add a new adapter crate under `adapters/` implementing `core::GameAdapter`.
2. Register the adapter in `server/src/main.rs`.
3. Add the mode to the client pregame selector in `client/src/routes/+page.svelte`.
