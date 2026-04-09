# Edif.io Monorepo Agent Guide

## Architecture
- Monorepo: one Rust server binary (`server`), one SvelteKit client (`client`).
- Game modes are adapters loaded at startup, not separate processes.

## Structure
- `client`: SvelteKit frontend.
- `core`: shared Rust protocol, rules, and websocket runtime.
- `adapters/*`: game-mode adapters (prompt/validation/scoring).
<!-- GENERATED:REPO_ADAPTER_LIST -->
  - `adapters/arithmetic`: Implements arithmetic prompt racing for edif.io.
  - `adapters/keyboarding`: Implements word-based prompt racing for edif.io.
  - `adapters/state-abbreviations`: state-abbreviations adapter.
<!-- /GENERATED:REPO_ADAPTER_LIST -->
- `server`: single Rust binary hosting all adapters.

## Boundaries
- Server authoritative: room state, prompts, scoring, sizes, win conditions.
- Client authoritative: visual blob positioning only.
- Adapters own: prompt generation and answer correctness.

## Conventions
- WebSocket payloads: JSON, `type` field in camelCase.
- Never add server-authoritative blob positions to client.
- Additive adapter interfaces; new modes must not break existing ones.

## AGENTS.md Maintenance
- Some AGENTS.md files contain `<!-- GENERATED:... -->` marker sections that are auto-filled by `scripts/generate-agents-md.sh`.
- Edit content **outside** those markers freely; content **inside** markers will be overwritten on the next generation run.
- After adding or renaming an adapter, run `bash scripts/generate-agents-md.sh` to update generated sections.
