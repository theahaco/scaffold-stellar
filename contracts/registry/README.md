# loam-registry

`loam-registry` is a Rust crate for managing and deploying smart contracts on the Soroban blockchain. It provides an easy-to-use interface for developers to interact with the blockchain and deploy their smart contracts without dealing with low-level implementation details.

## Features

- Register contract names for publishing
- Publish contract binaries with version management
- Fetch contract binaries and metadata
- Deploy published contracts to the blockchain
- Retrieve deployment statistics for contracts
- Manage contract ownership and redeployment

## Usage

Add `loam-registry` as a dependency in your project's `Cargo.toml` file:

```toml
[dependencies]
loam-registry = "0.0.0"
