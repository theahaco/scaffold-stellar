# stellar-scaffold-cli

CLI toolkit for Stellar smart contract development, providing project scaffolding, build automation, and development workflow tools.

Stellar Scaffold CLI comes with three main commands:

* `stellar scaffold init` - Creates a new Stellar smart contract project with best practices and configurations in place, including an `environments.toml` file for managing network settings, accounts, and contracts across different environments.

* `stellar scaffold build` - Manages two key build processes:
  * Build smart contracts with metadata and handle dependencies
  * Generate TypeScript client packages for frontend integration
  
  The build process respects environment configurations from `environments.toml` and handles contract deployment states based on the current environment (controlled via `STELLAR_SCAFFOLD_ENV`).

* `stellar scaffold watch` - Development mode that monitors contract source files and `environments.toml` for changes, automatically rebuilding as needed. Defaults to using the `development` environment.

## Getting Started

1. Install the CLI:
```bash
cargo install --git https://github.com/ahalabs/scaffold-stellar stellar-scaffold-cli
```

2. Create a new project:
```bash
stellar scaffold init my-project
cd my-project
```

This creates:
- A smart contract project with recommended configurations
- A frontend application based on [scaffold-stellar-frontend](https://github.com/AhaLabs/scaffold-stellar-frontend)
- Environment configurations for both contract and frontend development

3. Set up your environment:
```bash
cp .env.example .env
```

4. Start development:
```bash
stellar scaffold watch --build-clients
```

## Environment Configuration

Projects use `environments.toml` to define network settings, accounts, and contract configurations for different environments. Example:

```toml
[development]
network = { 
    name = "standalone",
    run_locally = true
}
accounts = ["account1", "account2"]

[staging]
network = { 
    name = "testnet"
}

[production]
network = { 
    name = "mainnet"
}
```

## Build Process Details

`stellar scaffold build` and `stellar scaffold watch` manage:

1. Smart contract compilation and deployment based on environment
2. TypeScript client package generation for frontend integration
3. Network and account management (create/fund accounts in development)
4. Contract initialization via constructor args and post-deploy scripts

The build process ensures:
- Correct dependency resolution and build order
- Environment-specific contract deployments
- TypeScript client generation for frontend integration
- Contract state verification and updates

### Setting contract metadata

Contract metadata is set when running `stellar scaffold build` and adds the following fields from your contract's Cargo.toml to the contract metadata.

| Metadata   | Cargo.toml Key (under `package`) | Description                                      |
|------------|----------------|--------------------------------------------------|
| `name`     | `name`         | The name of the contract                         |
| `binver`   | `version`      | The version of the WASM bytecode of the contract                      |
| `authors`  | `authors`      | The author(s) of the contract, often with an email  |
| `home_domain` | `homepage`  | The relevant domain to relate to this contract   |
| `source_repo` | `repository` | The source repository URL for the contract      |

## Environment Variables

- `STELLAR_SCAFFOLD_ENV`: Sets current environment (development/staging/production)
- `STELLAR_ACCOUNT`: Default account for transactions
- `STELLAR_RPC_URL`: RPC endpoint URL
- `STELLAR_NETWORK_PASSPHRASE`: Network passphrase

## For More Information

See the full documentation:
- [CLI Commands Guide](https://github.com/ahalabs/scaffold-stellar/blob/main/docs/cli.md)
- [Environment Configuration](https://github.com/ahalabs/scaffold-stellar/blob/main/docs/environments.md)
- [Deployment Guide](https://github.com/ahalabs/scaffold-stellar/blob/main/docs/deploy.md)