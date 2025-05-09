# stellar-registry-cli

Command line interface for managing smart contract deployments through the Stellar Registry system. This tool enables deploying and installing contracts that have been published to the Stellar Registry.

## Installation

Install directly from the repository:

```bash
cargo install --git https://github.com/ahalabs/scaffold-stellar stellar-registry-cli
```

## Commands

### Deploy

Deploy a published contract with optional initialization parameters:

```bash
stellar registry deploy \
  --deployed-name <NAME> \
  --published-name <CONTRACT> \
  [CONTRACT_FN_AND_ARGS...]
```

Options:
- `--deployed-name`: Name to give this contract instance
- `--published-name`: Name of the published contract to deploy
- `CONTRACT_FN_AND_ARGS`: Optional initialization function and arguments

### Install

Install a deployed contract's Wasm and contract ID locally:

```bash
stellar registry install <DEPLOYED_NAME> \
  --out-dir <PATH>
```

Options:
- `DEPLOYED_NAME`: Name of the deployed contract to install
- `--out-dir (-o)`: Directory to save the contract files
  - Creates: `<out_dir>/<name>.wasm` and `<out_dir>/contract_id.txt`

## Configuration

The CLI can be configured through environment variables:

- `STELLAR_REGISTRY_CONTRACT_ID`: Override the default registry contract ID
- `SOROBAN_NETWORK`: Network to use (e.g., "testnet")
- `SOROBAN_RPC_URL`: Custom RPC endpoint (default: https://soroban-testnet.stellar.org:443)
- `SOROBAN_NETWORK_PASSPHRASE`: Network passphrase (default: Test SDF Network ; September 2015)

## Example Usage

1. Deploy a token contract:
```bash
stellar registry deploy \
  --deployed-name my-token \
  --published-name token \
  initialize \
  --name "My Token" \
  --symbol "MTK" \
  --decimals 7
```

2. Install the deployed contract:
```bash
stellar registry install my-token \
  --out-dir ./contracts/my-token
```

## Registry Contract

The registry contract is deployed on testnet at:
```
CC2FLEJRHB2Q5JOAJNPFZU25ZAY6IFYXJL7UQ5GF36F3G5QZ4CQILUID
```

## See Also

- [Registry Guide](../../docs/registry.md) - Detailed guide on using the registry system
- [Environment Configuration](../../docs/environments.md) - Configuration details for different networks