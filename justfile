set dotenv-load

export PATH := './target/bin:' + env_var('PATH')
export CONFIG_DIR := 'target/'


[private]
path:
    just --list

scaffold +args:
    @cargo run --bin stellar-scaffold --quiet -- {{args}}

registry +args:
    @cargo run --bin stellar-registry --quiet -- {{args}}

stellar-scaffold +args:
    @cargo run --bin stellar-scaffold -- {{args}}

s +args:
    @stellar {{args}}

stellar +args:
    @stellar {{args}}

build_contract p:
    stellar contract build --profile contracts --package {{p}}


# build contracts
build:
    just stellar-scaffold build
    cargo build --package stellar-registry-cli
    stellar contract optimize --wasm ./target/stellar/local/registry.wasm --wasm-out ./target/stellar/local/registry.wasm

# Setup the project to use a pinned version of the CLI
setup:
    git config core.hooksPath .githooks
    -cargo binstall -y stellar-cli --version 23.1.3 --install-path ./target/bin

# Build stellar-scaffold-cli test contracts to speed up testing
build-cli-test-contracts:
    just stellar-scaffold build --manifest-path crates/stellar-scaffold-test/fixtures/soroban-init-boilerplate/Cargo.toml

test: build
    cargo nextest run -E 'package(stellar-scaffold-cli)'
    cargo nextest run -E 'package(stellar-registry-cli)'
    cargo nextest run

test-integration: build-cli-test-contracts
    cargo nextest run --verbose --package stellar-scaffold-cli --features integration-tests --no-run
    cargo nextest run --verbose --package stellar-registry-cli --features integration-tests --no-run
    cargo nextest run --package stellar-registry-cli --features integration-tests
    cargo nextest run --package stellar-scaffold-cli --features integration-tests

create: build
    rm -rf .soroban
    -stellar keys generate default --fund
    # just stellar contract deploy --wasm ./target/stellar/local/example_status_message.wasm --alias core --source-account default
