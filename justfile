set dotenv-load := true

export PATH := './target/bin:' + env_var('PATH')
export CONFIG_DIR := 'target/'

[private]
path:
    just --list

scaffold +args:
    @cargo run --bin stellar-scaffold --quiet -- {{ args }}

registry +args:
    @cargo run --bin stellar-registry --quiet -- {{ args }}

stellar-scaffold +args:
    @cargo run --bin stellar-scaffold -- {{ args }}

s +args:
    @stellar {{ args }}

stellar +args:
    @stellar {{ args }}

build_contract p:
    stellar contract build --profile contracts --package {{ p }}

# build contracts
build:
    just stellar-scaffold build --profile contracts
    cargo build --package stellar-registry-cli
    stellar contract optimize --wasm ./target/stellar/local/registry.wasm --wasm-out ./target/stellar/local/registry.wasm

# Setup the project to use a pinned version of the CLI
setup:
    git config core.hooksPath .githooks
    -cargo binstall -y stellar-cli --version 23.3.0 --force --install-path ./target/bin

# Build stellar-scaffold-cli test contracts to speed up testing
build-cli-test-contracts:
    just stellar-scaffold build --manifest-path crates/stellar-scaffold-test/fixtures/soroban-init-boilerplate/Cargo.toml

test: build
    cargo t -E 'package(stellar-scaffold-cli)'
    cargo t

test-integration: build-cli-test-contracts
    just test-integration-scaffold-contracts
    just test-integration-scaffold-features
    just test-integration-scaffold-examples-1
    just test-integration-scaffold-examples-2
    just test-integration-registry

[private]
_test-scaffold filter:
    cargo t --package stellar-scaffold-cli --features integration-tests -E '{{ filter }}'

# Run scaffold-cli accounts & contracts integration tests
test-integration-scaffold-contracts:
    just _test-scaffold 'test(build_clients::accounts::) or test(build_clients::contracts::)'

# Run scaffold-cli init_script, network, watch & clean integration tests
test-integration-scaffold-features:
    just _test-scaffold 'not test(build_clients::accounts::) and not test(build_clients::contracts::) and not test(examples::)'

# Run scaffold-cli example integration tests (cases 1-14)
test-integration-scaffold-examples-1:
    just _test-scaffold 'test(examples::) and (test(/case_0/) or test(/case_1[0-4]/))'

# Run scaffold-cli example integration tests (cases 15-27)
test-integration-scaffold-examples-2:
    just _test-scaffold 'test(examples::) and (test(/case_1[5-9]/) or test(/case_2/))'

# Run registry-cli integration tests
test-integration-registry:
    cargo t --package stellar-registry-cli --features integration-tests

create: build
    rm -rf .soroban
    -stellar keys generate default --fund
    # just stellar contract deploy --wasm ./target/stellar/local/example_status_message.wasm --alias core --source-account default

clippy *args:
    cargo clippy --all {{ args }} \
    -- -Dclippy::pedantic -Aclippy::must_use_candidate -Aclippy::missing_errors_doc -Aclippy::missing_panics_doc

clippy-test:
    just clippy --tests
