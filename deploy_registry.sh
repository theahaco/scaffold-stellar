#!/usr/bin/env bash
set -e

PATH=./target/bin:$PATH

# sha256 -s verified
#

registry_version() {
     awk -F'"' '/^version[[:space:]]*=/ { print $2; exit }' \
         "$(dirname "$0")/contracts/registry/Cargo.toml"
}
VERSION=v$(registry_version)
curl -L https://github.com/theahaco/scaffold-stellar/releases/download/registry-$VERSION/registry_$VERSION.wasm > ./target/stellar/registry_$VERSION.wasm

VERIFED=$(sha256 -s v0.5.0)
ADMIN=theahaco
ADDRESS=GAMPJROHOAW662FINQ4XQOY2ULX5IEGYXCI4SMZYE75EHQBR6PSTJG3M
echo "$VERIFED"

stellar contract deploy --alias registry \
                        --wasm ./target/stellar/registry_$VERSION.wasm \
                        --source "$ADMIN" \
                        --salt $VERIFED \
                        -- \
                        --admin "$ADMIN" \
                        --manager "\"$ADDRESS\""




just registry publish  --wasm ./target/stellar/registry_$VERSION.wasm \
                         --author "$ADMIN" \
                         --source "$ADMIN"
