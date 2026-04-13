# stellar-registry

The missing infrastructure layer between "I wrote a smart contract" and "the ecosystem can safely use my smart contract."

- Smart Contract source code: this repository; this folder
- Frontend source code: [theahaco/registry-ui](https://github.com/theahaco/registry-ui)
- Indexer & API: [theahaco/registry-indexer](https://github.com/theahaco/registry-indexer)

`stellar-registry` is a Rust crate for managing and deploying smart contracts on the Soroban blockchain. It provides an easy-to-use interface for developers to interact with the blockchain and deploy their smart contracts without dealing with low-level implementation details.

## Features

- Register contract names for publishing
- Publish contract binaries with version management
- Fetch contract binaries and metadata
- Deploy published contracts to the blockchain
- Retrieve deployment statistics for contracts
- Manage contract ownership and redeployment
