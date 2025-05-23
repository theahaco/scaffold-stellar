set dotenv-load

export PATH := './target/bin:' + env_var('PATH')
export CONFIG_DIR := 'target/'
# hash := `soroban contract install --wasm ./target/wasm32-unknown-unknown/contracts/example_status_message.wasm`



# list all
[private]
path:
    just --list

stellar-scaffold +args:
    @cargo run --quiet --bin stellar-scaffold -- {{args}}

s +args:
    @stellar {{args}}

stellar +args:
    @stellar {{args}}

build_contract p:
    stellar contract build --profile contracts --package {{p}}

# build contracts
build:
    just stellar-scaffold build
    just stellar contract optimize --wasm target/stellar/stellar_registry_contract.wasm --wasm-out target/stellar/stellar_registry_contract.wasm
    just stellar contract optimize --wasm target/stellar/stellar_registry_tools_contract.wasm --wasm-out target/stellar/stellar_registry_tools_contract.wasm

# Setup the project to use a pinned version of the CLI
setup:
    -cargo install stellar-cli --debug --version 22.8.0 --root ./target

# Build stellar-scaffold-cli test contracts to speed up testing
build-cli-test-contracts:
    just stellar-scaffold build --manifest-path crates/stellar-scaffold-cli/tests/fixtures/soroban-init-boilerplate/Cargo.toml

test: build
    cargo nextest run --workspace

test-integration: build-cli-test-contracts
    cargo nextest run -E 'package(stellar-scaffold-cli)' --features integration-tests

create: build
    rm -rf .soroban
    -stellar keys generate default --fund
    #just stellar contract deploy --wasm ./target/stellar/example_status_message.wasm --alias core --source-account default