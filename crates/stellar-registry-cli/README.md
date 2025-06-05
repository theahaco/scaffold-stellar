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
  --network <NETWORK_NAME> \
  --contract-name <NAME> \
  --wasm-name <NAME> \
  -- \
  [CONSTRUCTOR_ARGS...]
```

Options:
- `--contract-name`: Name to give this contract instance
- `--wasm-name`: Name of the published contract to deploy
- `CONSTRUCTOR_ARGS`: Optional arguments if contract implements `__constructor` to deploy and initialize the contract

### Install

Install a deployed as an alias to be used by `stellar-cli`:

```bash
stellar registry install <CONTRACT_NAME>
```

Options:
- `CONTRACT_NAME`: Name of the deployed contract to install

## Configuration

`stellar-cli` provides a way to use a default config for accounts and networks:
```bash
stellar keys use alice
```

```bash
stellar network use testnet
```

The CLI can be configured through environment variables:

- `STELLAR_REGISTRY_CONTRACT_ID`: Override the default registry contract ID
- `STELLAR_NETWORK`: Network to use (e.g., "testnet", "mainnet")
- `STELLAR_RPC_URL`: Custom RPC endpoint (default: https://soroban-testnet.stellar.org:443)
- `STELLAR_NETWORK_PASSPHRASE`: Network passphrase (default: Test SDF Network ; September 2015)
- `STELLAR_ACCOUNT`: Source account to use

These variables can also be in a `.env` file in the current working directory.

## Example Usage

1. Deploy a token contract:
```bash
stellar registry deploy \
  --contract-name my-token \
  --wasm-name token \
  initialize \
  --name "My Token" \
  --symbol "MTK" \
  --decimals 7
```

2. Install the deployed contract:
```bash
stellar registry install my-token
```

Then can interact with it the contract with `stellar-cli`:
```bash
stellar contract invoke --id my-token -- --help
```

### Transitioning to Mainnet

Once you are satisfied with your contract you can publish and deploy on Mainnet.

The first step is adding Mainnet to your `stellar-cli`. [See the reccommend list of RPC provides here]( https://developers.stellar.org/docs/data/rpc/rpc-providers)

Then you must add it with the following command:
```bash
stellar network add mainnet --network-passphrase "Public Global Stellar Network ; September 2015" --rpc-url <FROM_LIST>
```
Then make it your default
```bash
stellar network use mainnet
```
or if using a `bash` like shell to set it for just the current session:
```bash
export STELLAR_NETWORK=mainnet
```
or if while in the current directory:
```bash
echo STELLAR_NETWORK=mainnet >> .env
```

## Publishing and then deploying

Publishing and deploying are exactly the same!


## See Also

- [Registry Guide](../../docs/registry.md) - Detailed guide on using the registry system
- [Environment Configuration](../../docs/environments.md) - Configuration details for different networks