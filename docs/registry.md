# Registry Guide

The Stellar Registry is a system for publishing, deploying, and managing smart contracts on the Stellar network. This guide explains how to use the registry CLI tools to manage your contracts.

## Overview

The registry system consists of two main components:

1. The on-chain registry contract (deployed on testnet)
2. The `stellar-registry` CLI tool for interacting with the registry

## Prerequisites

- Install the registry CLI:
```bash
cargo install --git https://github.com/ahalabs/scaffold-stellar stellar-registry-cli
```

## Commands

### Deploy Contract

Deploy a published contract with custom initialization parameters:

```bash
stellar registry deploy \
  --deployed-name my-contract-instance \
  --published-name my-contract \
  [CONTRACT_FUNCTION_AND_ARGS...]
```

Options:
- `--deployed-name`: The name to give this contract instance
- `--published-name`: The name of the previously published contract to deploy
- `CONTRACT_FUNCTION_AND_ARGS`: Optional initialization function and arguments

### Install Contract

Install a deployed contract locally:

```bash
stellar registry install <DEPLOYED_NAME> \
  --out-dir path/to/output
```

Options:
- `DEPLOYED_NAME`: Name of the deployed contract to install
- `--out-dir (-o)`: Directory to save the contract WASM and ID files

## Configuration

The registry CLI respects the following environment variables:

- `STELLAR_REGISTRY_CONTRACT_ID`: Override the default registry contract ID
- `SOROBAN_NETWORK`: Network to use (e.g., "testnet")
- `SOROBAN_RPC_URL`: Custom RPC endpoint
- `SOROBAN_NETWORK_PASSPHRASE`: Custom network passphrase

Default values:
- RPC URL: `https://soroban-testnet.stellar.org:443`
- Network Passphrase: `Test SDF Network ; September 2015`

## Example Workflow

1. Deploy a new contract instance:
```bash
stellar registry deploy \
  --deployed-name token-a \
  --published-name token \
  initialize \
  --name "Token A" \
  --symbol "TKNA" \
  --decimals 7
```

2. Install the deployed contract locally:
```bash
stellar registry install token-a \
  --out-dir ./contracts/token-a
```

This will:
- Save the contract WASM to `./contracts/token-a/token-a.wasm`
- Save the contract ID to `./contracts/token-a/contract_id.txt`

## Best Practices

1. Use descriptive deployed names that reflect the contract's purpose
2. Keep track of deployed contract IDs by saving them in version control
3. Document initialization parameters used for each deployment
4. Use environment variables for different network configurations

## Reference

The registry contract is deployed at:
```
CC2FLEJRHB2Q5JOAJNPFZU25ZAY6IFYXJL7UQ5GF36F3G5QZ4CQILUID
```

For more detailed information about the available commands:
```bash
stellar registry --help
```
