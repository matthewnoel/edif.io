# edif.io

Pluggable multiplayer prompt-race game. **[Play Now](https://edif.io)**

## Ethos

- **Everyone keeps playing**
  - Whether you're leading or trailing, the game keeps you answering and practicing — nobody sits idle.
- **Catch-up, not punishing**
  - Power-ups give trailing players a boost without stopping leaders from practicing, keeping rounds fun and competitive for all skill levels.
- **Safe by default**
  - No custom player names or user-generated text — designed so a teacher or parent never has to worry about what's on screen.

## Developing

### Prerequisites

 - [Rust](https://rust-lang.org/tools/install/)
 - [node.js](https://github.com/nvm-sh/nvm)

### Running Locally

```sh
make dev
```

### Validation, Testing & Formatting

```sh
make check
make test
make format
```

### Deployment

The app is containerized via the `Dockerfile` and uses Caddy as a reverse proxy.
The Node version is sourced from `client/.nvmrc` and passed as a Docker build arg.

```sh
make build
```

The image can be deployed to any Docker-capable host. A deployment is maintained by [Plunge Studios](https://github.com/plunge-studios) at [edif.io](https://edif.io), but you're free to make, modify, and deploy your own versions if you wish.
