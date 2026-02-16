set dotenv-load := true

export PATH := './target/bin:' + env_var('PATH')
export CONFIG_DIR := 'target/'
export CI_BUILD := env_var_or_default('CI_BUILD', '')

[private]
path:
    just --list

scaffold +args:
    @cargo run --bin stellar-scaffold --quiet -- {{ args }}

registry +args:
    @cargo run --bin stellar-registry --quiet -- {{ args }}

stellar-scaffold +args:
    @cargo run $CI_BUILD --bin stellar-scaffold -- {{ args }}

s +args:
    @stellar {{ args }}

stellar +args:
    @stellar {{ args }}

build_contract p:
    stellar contract build --profile contracts --package {{ p }}

# build contracts
build:
    just stellar-scaffold build --profile contracts
    cargo build $CI_BUILD --package stellar-registry-cli
    stellar contract optimize --wasm ./target/stellar/local/registry.wasm --wasm-out ./target/stellar/local/registry.wasm

# Setup the project to use a pinned version of the CLI
setup:
    git config core.hooksPath .githooks
    -cargo binstall -y stellar-cli --version 23.3.0 --force --install-path ./target/bin

# Build stellar-scaffold-cli test contracts to speed up testing
build-cli-test-contracts:
    just stellar-scaffold build --manifest-path crates/stellar-scaffold-test/fixtures/soroban-init-boilerplate/Cargo.toml

test: build
    cargo t

test-integration: build-cli-test-contracts
    cargo t --features integration-tests

[private]
_test-integration package filter ci="false":
    cargo t  -E 'package({{ package }}) and {{ filter }}' \
    {{ if ci == "false" { '--features integration-tests' } else { '--binaries-metadata target/nextest/binaries-metadata.json --cargo-metadata target/nextest/cargo-metadata.json --target-dir-remap target --workspace-remap .' } }}

[private]
_test-scaffold filter ci="false":
    just _test-integration stellar-scaffold-cli '{{ filter }}' {{ ci }}

[private]
_test-scaffold-ci filter:
    jsut _test-scaffold {{ filter }}  --binaries-metadata target/nextest/binaries-metadata.json --cargo-metadata target/nextest/cargo-metadata.json --target-dir-remap target --workspace-remap .

# Run scaffold-cli accounts & contracts integration tests
test-integration-scaffold-build-clients ci="false":
    just _test-scaffold 'test(build_clients)' {{ ci }}

# Run scaffold-cli init_script, network, watch & clean integration tests
test-integration-scaffold-features ci="false":
    just _test-scaffold 'test(features::)' {{ ci }}

# Run scaffold-cli example integration tests (cases 1-14)
test-integration-scaffold-examples-1 ci="false":
    just _test-scaffold 'test(examples::) and test(/case_01/)' {{ ci }}
    just _test-scaffold 'test(examples::) and (test(/case_0[2-9]/) or test(/case_1[0-4]/))' {{ ci }}

# Run scaffold-cli example integration tests (cases 15-27)
test-integration-scaffold-examples-2 ci="false":
    just _test-scaffold 'test(examples::) and test(/case_15/)' {{ ci }}
    just _test-scaffold 'test(examples::) and (test(/case_1[6-9]/) or test(/case_2/))' {{ ci }}

# Run registry-cli integration tests
test-integration-registry ci="false":
    just _test-integration stellar-registry-cli 'test(/./)' {{ ci }}

create: build
    rm -rf .soroban
    -stellar keys generate default --fund
    # just stellar contract deploy --wasm ./target/stellar/local/example_status_message.wasm --alias core --source-account default

clippy *args:
    cargo clippy --all {{ args }} \
    -- -Dclippy::pedantic -Aclippy::must_use_candidate -Aclippy::missing_errors_doc -Aclippy::missing_panics_doc

clippy-test:
    just clippy --tests --all-features
