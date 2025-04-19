#!/bin/bash
PATH=./target/bin:$PATH

stellar contract invoke -- admin_set --new_admin default
stellar contract invoke -- --help
WASM=$(stellar contract install --wasm ./target/loam/example_status_message.wasm)
echo $WASM
stellar contract invoke -- redeploy --wasm_hash "$WASM"
stellar contract invoke -- --help
stellar contract invoke -- --help | grep messages_get || exit 1
