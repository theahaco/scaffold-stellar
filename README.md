# Scaffold Stellar

[![Apache 2.0 licensed](https://img.shields.io/badge/license-apache%202.0-blue.svg)](LICENSE)
[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/AhaLabs/scaffold-stellar)

Scaffold Stellar is a convention-over-configuration toolkit for blockchain and distributed application development on the Stellar network. It provides a seamless development experience through CLI tools, smart contract management, and deployment utilities.

The project consists of several main components:

- **stellar-scaffold-cli**: The main command-line interface for Scaffold Stellar. It provides commands for initializing projects, managing development workflows, and automating contract deployment.

- **stellar-registry-cli**: A CLI tool for managing the on-chain contract registry, handling contract publishing, deployment, and version management.

- **stellar-build**: Core build utilities and helper functions for Stellar smart contract development.

- **stellar-scaffold-macro**: Rust procedural macros that simplify contract development and integrate with the Scaffold Stellar ecosystem.

- **registry**: The on-chain smart contract that powers the Scaffold Stellar registry system, enabling contract verification, naming, and version management.


## Features

- **CLI Plugins for Stellar**
  - `stellar scaffold`: Initialize and manage Scaffold Stellar projects
    - Creates smart contract projects with best practices
    - Includes frontend setup using [scaffold-stellar-frontend](https://github.com/AhaLabs/scaffold-stellar-frontend)
  - `stellar registry`: Publish Wasm binaries and deploy smart contracts
  - Automated development workflow with hot reloading

  Currently these are available as separate binaries: `stellar-scaffold` and `stellar-registry` respectively.

- **Declarative Environment Management**
  - Environment-specific builds (development, testing, staging, production)
  - Seamless integration with both local and deployed contracts
  - Network configuration via `environments.toml`

- **Coming soon: Smart Contract Registry**
  - On-chain publishing platform for Wasm binaries
  - Version management and contract naming
  - Contract verification and dependency management

- **Coming soon: Deployment Pipeline**
  - Streamlined deployment process for testnet and mainnet
  - Contract lifecycle management
  - Automated environment updates

## Project Structure

- `stellar-scaffold-cli`: Main CLI tool for project scaffolding and development
- `stellar-registry-cli`: Contract registry and deployment management
- `stellar-build`: Build utilities for Stellar smart contracts
- `stellar-scaffold-macro`: Procedural macros for contract development

## Installation

### Development Setup
```bash
just setup && just build
```

### Direct Installation
To install the executables directly:

```bash
# Install stellar-scaffold-cli
cargo install stellar-scaffold-cli

# Install stellar-registry-cli
cargo install stellar-registry-cli
```

## Quick Start

1. Install the required CLI tools:
```bash
# Install stellar-scaffold CLI
cargo install stellar-scaffold-cli

# Install registry CLI (needed for deployments)
cargo install stellar-registry-cli
```

2. Initialize a new project:
```bash
stellar scaffold init my-project
cd my-project
```

3. Set up your development environment:
```bash
# Copy and configure environment variables
cp .env.example .env

# Install frontend dependencies
npm install
```

4. Start development environment:
```bash
npm run dev
```

5. For testnet/mainnet deployment:
```bash
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

## Scaffold Initial Project Structure

When you run `stellar scaffold init`, it creates a frontend-focused project structure with example contracts:

```
my-project/                      # Your initialized project
├── contracts/                   # Example smart contracts
├── packages/                    # Auto-generated TypeScript clients
├── src/                         # Frontend React application
│   ├── components/              # React components
│   ├── contracts/               # Contract interaction helpers
│   ├── App.tsx                  # Main application component
│   └── main.tsx                 # Application entry point
├── target/                      # Build artifacts and WASM files
├── environments.toml            # Environment configurations
├── package.json                 # Frontend dependencies
└── .env                         # Local environment variables
```

This template provides a ready-to-use frontend application with example smart contracts and their TypeScript clients. You can use these as reference while building your own contracts and UI. The frontend is set up with Vite, React, and includes basic components for interacting with the contracts.

See the [CLI Documentation](./docs/cli.md) for detailed command information and the [Environments Guide](./docs/environments.md) for configuration details.

## Documentation

- [CLI Commands](./docs/cli.md)
- [Environment Configuration](./docs/environments.md)
- [Registry Guide](./docs/registry.md)
- [Deployment Guide](./docs/deployment.md)

## Contributing

Contributions are welcome! Please check out our [Contributing Guide](CONTRIBUTING.md) for guidelines.

## License

This project is licensed under the Apache-2.0 License - see the [LICENSE](LICENSE) file for details.
