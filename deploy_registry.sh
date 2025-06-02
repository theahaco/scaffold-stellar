#!/bin/bash
set -e

PATH=./target/bin:$PATH

stellar contract deploy --alias registry \
                        --wasm ./target/stellar/registry.wasm \
                        --source "$ADMIN" \
                        --salt 1 \
                        -- \
                        --admin "$ADMIN"

registry="stellar contract invoke --id registry --"

$registry --help

just registry publish --wasm ./target/stellar/registry.wasm \
                         --author "$ADMIN" \
                         --source "$ADMIN"
