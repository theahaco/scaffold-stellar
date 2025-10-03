# Registry Guide

The Stellar Registry is a system for publishing, deploying, and managing smart contracts on the Stellar network. This guide explains how to use the registry CLI tools to manage your contracts.

## Overview

The registry system consists of two main components:

1. The on-chain registry contract (deployed on testnet)
2. The `stellar-registry` CLI tool for interacting with the registry

## Prerequisites

- Install the registry CLI:
```bash
cargo install --git https://github.com/theahaco/scaffold-stellar stellar-registry-cli
```

## Commands

### Publish Contract

Publish a compiled contract to the Stellar Registry:

```bash
stellar registry publish \
  --wasm <PATH_TO_WASM> \
  [--author <AUTHOR_ADDRESS>] \
  [--wasm-name <NAME>] \
  [--binver <VERSION>] \
  [--dry-run]
```

Options:
- `--wasm`: Path to the compiled WASM file (required)
- `--author (-a)`: Author address (optional, defaults to the configured source account)
- `--wasm-name`: Name for the published contract (optional, extracted from contract metadata if not provided)
- `--binver`: Binary version (optional, extracted from contract metadata if not provided)
- `--dry-run`: Simulate the publish operation without actually executing it (optional)

### Deploy Contract

Deploy a published contract with optional initialization parameters:

```bash
stellar registry deploy \
  --contract-name <DEPLOYED_NAME> \
  --wasm-name <PUBLISHED_NAME> \
  [--version <VERSION>] \
  -- \
  [CONSTRUCTOR_FUNCTION] [CONSTRUCTOR_ARGS...]
```

Options:
- `--contract-name`: The name to give this contract instance (required)
- `--wasm-name`: The name of the previously published contract to deploy (required)
- `--version`: Specific version of the published contract to deploy (optional, defaults to most recent version)
- `CONSTRUCTOR_FUNCTION`: Optional constructor function name if contract implements initialization
- `CONSTRUCTOR_ARGS`: Optional arguments for the constructor function

Note: Use `--` to separate CLI options from constructor function and arguments.

## Configuration

The registry CLI respects the following environment variables:

- `STELLAR_REGISTRY_CONTRACT_ID`: Override the default registry contract ID
- `STELLAR_NETWORK`: Network to use (e.g., "testnet", "mainnet")
- `STELLAR_RPC_URL`: Custom RPC endpoint (default: https://soroban-testnet.stellar.org:443)
- `STELLAR_NETWORK_PASSPHRASE`: Network passphrase (default: Test SDF Network ; September 2015)
- `STELLAR_ACCOUNT`: Source account to use

These variables can also be in a `.env` file in the current working directory.

You can also configure `stellar-cli` defaults:
```bash
stellar keys use alice
stellar network use testnet
```

## Example Workflow

1. Publish a contract:
```bash
stellar registry publish \
  --wasm path/to/token.wasm \
  --wasm-name token \
  --binver "1.0.0"
```

2. Deploy the published contract with initialization:
```bash
stellar registry deploy \
  --contract-name my-token \
  --wasm-name token \
  --version "1.0.0" \
  -- \
  initialize \
  --name "My Token" \
  --symbol "MTK" \
  --decimals 7
```

3. Use the deployed contract with `stellar-cli`:
```bash
stellar contract invoke --id my-token -- --help
```

## Best Practices

1. Use descriptive contract and wasm names that reflect the contract's purpose
2. Follow semantic versioning for your contract versions
3. Always test deployments on testnet before mainnet
4. Use the `--dry-run` flag to simulate operations before executing them
5. Document initialization parameters used for each deployment
6. Use environment variables or `.env` files for different network configurations

## Registry Contract Addresses

The registry contract is deployed at different addresses for each network:

- **Testnet**: `CBCOGWBDGBFWR5LQFKRQUPFIG6OLOON35PBKUPB6C542DFZI3OMBOGHX`
- **Mainnet**: `CC3SILHAJ5O75KMSJ5J6I5HV753OTPWEVMZUYHS4QEM2ZTISQRAOMMF4`
- **Futurenet**: `CACPZCQSLEGF6QOSBF42X6LOUQXQB2EJRDKNKQO6US6ZZH5FD6EB325M`

## Troubleshooting

### Common Issues

1. **Contract name already exists**: Contract names must be unique within the registry. Choose a different name or check if you own the existing contract.

2. **Version must be greater than current**: When publishing updates, ensure the new version follows semantic versioning and is greater than the currently published version.

3. **Authentication errors**: Ensure your source account has sufficient XLM balance and is properly configured.

4. **Network configuration**: Verify your network settings match the intended deployment target (testnet vs mainnet).

For more detailed information about the available commands:
```bash
stellar registry --help
stellar registry <command> --help
```
