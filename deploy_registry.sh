#!/usr/bin/env bash
set -e

PATH=./target/bin:$PATH

# ifndef STELLAR_NETWORK
#    override STELLAR_NETWORK = local
# endif

VERIFED=$(sha256 -s verified)
UNVERIFIED=$(sha256 -s unverified)
ADMIN=theahaco
ADDRESS=GAMPJROHOAW662FINQ4XQOY2ULX5IEGYXCI4SMZYE75EHQBR6PSTJG3M

stellar contract deploy --alias registry \
                        --wasm ./target/stellar/local/registry.wasm \
                        --source "$ADMIN" \
                        --salt $VERIFED \
                        -- \
                        --admin "$ADMIN" \
                        --manager "\"$ADDRESS\""

stellar contract deploy --alias registry \
                        --wasm ./target/stellar/local/registry.wasm \
                        --source "$ADMIN" \
                        --salt $UNVERIFIED \
                        -- \
                        --admin "$ADMIN" 


registry="stellar contract invoke --id registry --"

$registry --help

just registry publish  --wasm ./target/stellar/local/registry.wasm \
                         --author "$ADMIN" \
                         --source "$ADMIN"
