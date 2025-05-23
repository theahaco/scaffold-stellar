#!/bin/bash
set -e

PATH=./target/bin:$PATH

stellar contract deploy --alias registry \
                        --wasm ./target/stellar/stellar_registry_contract.wasm \
                        -- \
                        --admin default

registry="stellar contract invoke --id registry --"

$registry --help

get-version () {
    cargo pkgid $1 | cut -d'@' -f2
}


for i in $(just build --ls); do
    echo "Publishing $i to registry $(get-version $i)"

done
