# Scaffold Stellar

[![Apache 2.0 licensed](https://img.shields.io/badge/license-apache%202.0-blue.svg)](LICENSE)
[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/AhaLabs/scaffold-stellar)

**Scaffold Stellar** is a developer toolkit for building decentralized applications (dApps) and smart contracts on the [**Stellar** blockchain](https://stellar.org).

It helps you go from **idea** to **working full-stack dApp** faster — by providing CLI tools, reusable contract templates, a smart contract registry, and a modern frontend.

---

## Why Use Scaffold Stellar?

- Simplifies blockchain dApp development
- Generates smart contract projects and React UIs
- Deploys smart contracts and manages versions
- Easy to learn for newcomers; powerful for pros

---

## What Is Stellar?

[**Stellar**](https://www.stellar.org/) is a blockchain designed for fast, low-cost financial transactions and smart contracts written in **Rust** and compiled to **WebAssembly (Wasm)**.

With Scaffold Stellar, you write smart contracts in Rust and interact with them using modern TypeScript + React tooling.

---

## Prerequisites

Before you begin, make sure you have the following installed:

| Tool | Description | Install Link |
|------|-------------|--------------|
| [Rust & Cargo](https://www.rust-lang.org/tools/install) | For writing and compiling smart contracts | `curl https://sh.rustup.rs -sSf \| sh` |
| [Node.js & npm](https://nodejs.org/) | For frontend development | Download from official site |
| [Stellar CLI](https://github.com/stellar/stellar-cli) | for building, deploying, and interacting with smart contracts | [`Link for the repo`](https://github.com/stellar/stellar-cli)

---

## **Quickstart** (New Developers Welcome!)

This section walks you through setting up Scaffold Stellar from scratch.

### 1. Install the Scaffold Stellar CLI

```
cargo install stellar-scaffold-cli
```

The Scaffold Stellar CLI is installed as a plugin under the `stellar` CLI.

### 2. Create a New Project
```
stellar scaffold init my-project
cd my-project
```

### 3. Configure Your Environment
```
# Copy and configure environment variables
cp .env.example .env
```

Edit `.env` with your preferred network, secret keys, and other settings.

### 4. Install Frontend Dependencies
```
# Install Frontend dependencies
npm install
```
### 5. Start Development
```
npm run dev
```
You should see your React frontend at http://localhost:5173.

### 6. For testnet/mainnet deployment:
```
# First publish your contract to the registry
stellar registry publish

# Then deploy an instance with constructor parameters
stellar registry deploy \
  --deployed-name my-contract \
  --published-name my-contract \
  -- \
  --param1 value1
  
# Can access the help docs with --help
stellar registry deploy \
  --deployed-name my-contract \
  --published-name my-contract \
  -- \
  --help

# Install the deployed contract locally
stellar registry install my-contract
```

## Project Layout
After scaffolding a project, your folder structure will look like this:

```
my-project/
├── contracts/            # Rust smart contracts (compiled to WASM)
├── packages/             # Auto-generated TypeScript contract clients
├── src/                  # React frontend code
│   ├── components/       # Reusable UI pieces
│   ├── contracts/        # Contract interaction logic
│   ├── App.tsx           # Main app component
│   └── main.tsx          # Entry point
├── environments.toml     # Configuration per environment (dev/test/prod)
├── .env                  # Local environment variables
├── package.json          # Frontend packages
├── target/               # Build outputs
```

This template provides a ready-to-use frontend application with example smart contracts and their TypeScript clients. You can use these as reference while building your own contracts and UI. The frontend is set up with `Vite`, `React`, and includes basic components for interacting with the contracts.

See the [CLI Documentation](https://github.com/AhaLabs/scaffold-stellar/blob/main/docs/cli.md) for detailed command information and the [Environments Guide](https://github.com/AhaLabs/scaffold-stellar/blob/main/docs/environments.md) for configuration details.

---

## CLI Tools
Scaffold Stellar provides two main CLI tools:

stellar-scaffold
Initialize and manage dApp projects:
```
stellar scaffold init my-project
stellar scaffold build
```
Manage contract deployment and versions:
```
stellar registry publish    # Publish contract to the registry
stellar registry deploy     # Deploy a contract instance
stellar registry install    # Install deployed contracts locally
```
> Use `--help` on any command for usage instructions.

---
## Smart Contract Deployment
1. Publish Your Contract
```
stellar registry publish
```
2. Deploy the Contract
```
stellar registry deploy \
  --deployed-name my-contract \
  --published-name my-contract \
  -- \
  --param1 value1
```
3. Install the Deployed Contract
```
stellar registry install my-contract
```
> You can deploy to testnet or mainnet depending on your `.env` and `environments.toml`.

---
## Concept: What Is the Contract Registry?
The registry is an on-chain smart contract that lets you:
* Publish and verify other contracts
* Manage multiple versions
* Reuse deployed contracts across dApps

>This means your contracts can be upgraded, shared, and used like packages.
---
## Project Structure (Top-Level)
Your repo contains the following key folders:

|Folder	| Purpose |
|-------|---------|
|`.cargo/`, `.config/`	| Rust and build settings| 
|`contracts/` |	Example smart contracts|
|`crates/`|	Internal Rust libraries and helpers|
|`docs/`|	Documentation files|
|`npm/`|	Shared frontend packages|
|`deploy_registry.sh`|	Helper script to deploy the registry|
|`justfile` |	Commands you can run with just|

---

Documentation 
* [CLI Commands](https://github.com/AhaLabs/scaffold-stellar/blob/main/docs/cli.md)
* [Environment Setup](https://github.com/AhaLabs/scaffold-stellar/blob/main/docs/environments.md)
* [Registry Guide](https://github.com/AhaLabs/scaffold-stellar/blob/main/docs/registry.md)

---
## Additional Developer Resources
- Video: [Intro to Scaffold Stellar](https://www.youtube.com/watch?v=559ht4K4pkM)
- Video: [Which Frontend?](https://www.youtube.com/watch?v=pz7O54Oia_w)
- Video: [Get Started Building](https://www.youtube.com/watch?v=H-M962aPuTk)
- Video: [Live Demo of Scaffold Stellar](https://www.youtube.com/watch?v=0syGaIn3ULk) 👈 Start Here

---
## Contributing
We love contributions! If you’re new, check these out:

[Contributing Guide](https://github.com/AhaLabs/scaffold-stellar/blob/main/CONTRIBUTING.md)

## License

#### This project is licensed under the Apache-2.0 License — see the [LICENSE](https://github.com/scaffold-stellar/blob/main/LICENSE) file for details.
---

## Need Help?
If you’re new to Stellar, Rust, or smart contracts:

Ask questions in the repo Discussions tab

Search [DeepWiki](https://deepwiki.org/)

Or just open an issue — we're happy to help!

Happy hacking! 
---
