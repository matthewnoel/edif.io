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

```sh
make dev
```

## Test and Validation

```sh
make check
```

```sh
make test
```

## Adding a New Game Mode

1. Add a new adapter crate under `adapters/` implementing `core::GameAdapter`.
2. Register the adapter in `server/src/main.rs`.
3. Add the mode to the client pregame selector in `client/src/routes/+page.svelte`.

## Current Game Modes

 - Keyboarding: *Type the global prompt correctly first to gain points.*
 - Arithmetic: *Proof of concept intended to implement the functionality of: [https://github.com/matthewnoel/arithmetic-practice](https://github.com/matthewnoel/arithmetic-practice)*

 ## Deployment

The app is containerized via the `Dockerfile` and uses Caddy as a reverse proxy.
It can be deployed to any Docker-capable host. A deployment is maintained by Plunge Studios at [edif.io](https://edif.io), but you're free to make, modify, and deploy your own versions if you wish.
