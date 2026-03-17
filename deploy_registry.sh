#!/usr/bin/env bash
set -e

PATH=./target/bin:$PATH

# sha256 -s verified
# echo -n verified | sha256sum
VERIFED=1c34f88707b55e6104c4eb20e71ffa3d33e414b71ef689a15fad0640d0ac58cb
# sha256 -s unverified
# echo -n unverified | sha256sum
UNVERIFIED=97b7e2db799e2b79e65f418b42a7d3054c95b2d3ab1dba243039597e44a38084
ADMIN=theahaco
ADDRESS=GAMPJROHOAW662FINQ4XQOY2ULX5IEGYXCI4SMZYE75EHQBR6PSTJG3M

stellar contract deploy --alias registry \
                        --wasm ./target/stellar/local/registry.wasm \
                        --source "$ADMIN" \
                        --salt $VERIFED \
                        -- \
                        --admin "$ADMIN" \
                        --manager "\"$ADDRESS\"" \
                        --is-root true


registry="stellar contract invoke --id registry --"

$registry --help

just registry publish  --wasm ./target/stellar/local/registry.wasm \
                         --author "$ADMIN" \
                         --source "$ADMIN"
