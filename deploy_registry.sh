#!/usr/bin/env bash
set -e

PATH=./target/bin:$PATH

curl -L https://github.com/theahaco/scaffold-stellar/releases/download/registry-v0.4.1/registry_v0.4.1.wasm > ./target/stellar/registry.wasm


# sha256 -s verified

VERIFED=$(sha256 -s v0.4.1)
ADMIN=theahaco
ADDRESS=GAMPJROHOAW662FINQ4XQOY2ULX5IEGYXCI4SMZYE75EHQBR6PSTJG3M
echo "$VERIFED"

stellar contract deploy --alias registry \
                        --wasm ./target/stellar/registry.wasm \
                        --source "$ADMIN" \
                        --salt $VERIFED \
                        -- \
                        --admin "$ADMIN" \
                        --manager "\"$ADDRESS\"" \
                        --is-root true


registry="stellar contract invoke --id registry --"



just registry publish  --wasm ./target/stellar/registry.wasm \
                         --author "$ADMIN" \
                         --source "$ADMIN"
