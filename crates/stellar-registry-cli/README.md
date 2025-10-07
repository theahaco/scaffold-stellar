# stellar-registry-cli

Command line interface for managing smart contract deployments through the Stellar Registry system. This tool enables publishing, deploying, and installing contracts that have been published to the Stellar Registry.

## Installation

Install from cargo:

```bash
cargo install stellar-registry-cli
```

Or [`cargo-binstall`](github.com/cargo-bins/cargo-binstall):

```bash
cargo binstall stellar-registry-cli
```

## Commands

### Publish

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

### Deploy

Deploy a published contract with optional initialization parameters:
```bash
stellar registry deploy \
  --contract-name <NAME> \
  --wasm-name <NAME> \
  [--version <VERSION>] \
  -- \
  [CONSTRUCTOR_FUNCTION] [CONSTRUCTOR_ARGS...]
```

Options:
- `--contract-name`: Name to give this contract instance (required)
- `--wasm-name`: Name of the published contract to deploy (required)
- `--version`: Specific version of the published contract to deploy (optional, defaults to most recent version)
- `CONSTRUCTOR_FUNCTION`: Optional constructor function name if contract implements initialization
- `CONSTRUCTOR_ARGS`: Optional arguments for the constructor function

Note: Use `--` to separate CLI options from constructor function and arguments.

### Install

Install a deployed contract as an alias to be used by `stellar-cli`:
```bash
stellar registry install <CONTRACT_NAME>
```

Options:
- `CONTRACT_NAME`: Name of the deployed contract to install (required)

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

1. Publish a contract:
```bash
stellar registry publish \
  --wasm path/to/my_token.wasm \
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

3. Install the deployed contract:
```bash
stellar registry install my-token
```

Then can interact with the contract with `stellar-cli`:
```bash
stellar contract invoke --id my-token -- --help
```

### Transitioning to Mainnet

Before you are ready to publish and deploy your contract on mainnet you need to be confident that it is safe.

## Contract Security

Make sure you are following [security best practices recommended by the stellar docs.](https://developers.stellar.org/docs/build/security-docs)

### Security tooling

You can use [scout soroban](https://github.com/CoinFabrik/scout-soroban) to statically analyze your code for potential security issues.

### Contract Auditing

For an additional level of security you can get your contract audited. Stellar has an [Audit Bank](https://stellar.org/blog/developers/soroban-security-audit-bank-raising-the-standard-for-smart-contract-security) that will help connect you with experienced audit providers and help cover the costs of the audit. See [here if you qualify.](https://stellarcommunityfund.gitbook.io/scf-handbook/supporting-programs/audit-bank/official-rules)

### Publishing to Mainnet

Once you are satisfied with your contract you can publish and deploy on Mainnet.

The first step is adding Mainnet to your `stellar-cli`. [See the recommended list of RPC providers here](https://developers.stellar.org/docs/data/rpc/rpc-providers)

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

Publishing and deploying are exactly the same as other networks, except now you use real lumens!

## See Also

- [Registry Guide](../../docs/docs/registry.md) - Detailed guide on using the registry system
- [Environment Configuration](../../docs/docs/environments.md) - Configuration details for different networks