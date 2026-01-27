# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Scaffold Stellar is a developer toolkit for building dApps and smart contracts on the Stellar blockchain. It provides two Stellar CLI plugins:
- **stellar-scaffold** (`crates/stellar-scaffold-cli`) - Project scaffolding, building, and frontend generation
- **stellar-registry** (`crates/stellar-registry-cli`) - Publishing and deploying contracts via an on-chain registry

## Common Commands

```bash
# Setup development environment (installs stellar-cli v23.3.0)
just setup

# Build contracts and optimize registry wasm
just build

# Run unit tests (builds first)
just test

# Run integration tests (requires local Stellar RPC via Docker)
just test-integration

# Format check
cargo fmt --all -- --check

# Lint (requires contracts built first)
cargo clippy --all

# Run CLI directly during development
just scaffold <args>    # runs stellar-scaffold
just registry <args>    # runs stellar-registry
```

## Architecture

### Crate Structure

| Crate | Purpose |
|-------|---------|
| `stellar-scaffold-cli` | Main CLI: init, build, generate, watch commands |
| `stellar-registry-cli` | Registry CLI: publish, deploy, download, upgrade commands |
| `stellar-build` | Contract building logic and dependency resolution |
| `stellar-registry-build` | Registry interaction and contract deployment logic |
| `stellar-registry` | Shared registry types and utilities |
| `stellar-scaffold-macro` | Procedural macros |
| `stellar-scaffold-test` | Test utilities and fixture contracts |

### Key Contracts

- `contracts/registry` - The on-chain Registry contract (manages wasm publication and contract deployment)
- `contracts/test/*` - Test contracts for integration testing
- `crates/stellar-scaffold-test/fixtures/` - Fixture contracts for CLI testing

### CLI Command Flow

**stellar-scaffold commands:** init → build → generate → watch
- `init` - Scaffolds new project from template (uses degit to fetch from scaffold-stellar-frontend repo)
- `build` - Builds contracts and generates TypeScript clients based on `environments.toml`
- `generate contract` - Adds new contract to existing project
- `watch` - Monitors and rebuilds on changes

**stellar-registry commands:** publish → deploy → create-alias
- `publish` - Uploads wasm to registry with semantic versioning
- `deploy` - Instantiates a published wasm as a named contract
- `create-alias` - Creates local stellar contract alias from registry

## Testing

- Unit tests run without external dependencies: `cargo t`
- Integration tests require local Stellar RPC running via Docker (stellar/quickstart image)
- Feature flag `integration-tests` enables RPC-dependent tests
- Test fixtures in `crates/stellar-scaffold-test/fixtures/`

## Build Profile

Contracts use a custom `[profile.contracts]` with aggressive optimization:
- `opt-level = "z"` (size optimization)
- `lto = true`
- `strip = "symbols"`
- Build with: `stellar contract build --profile contracts`
