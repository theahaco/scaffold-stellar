#!/bin/bash
set -e

PATH=./target/bin:$PATH

ifndef STELLAR_NETWORK
   override STELLAR_NETWORK = local
endif

stellar contract deploy --alias registry \
                        --wasm ./target/stellar/$(STELLAR_NETWORK)/registry.wasm \
                        --source "$ADMIN" \
                        --salt 0 \
                        -- \
                        --admin "$ADMIN"

registry="stellar contract invoke --id registry --"

$registry --help

just registry publish --wasm ./target/stellar/$(STELLAR_NETWORK)/registry.wasm \
                         --author "$ADMIN" \
                         --source "$ADMIN"
