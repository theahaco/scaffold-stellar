#!/usr/bin/env bash
set -euo pipefail

ADMIN="\"GAMPJROHOAW662FINQ4XQOY2ULX5IEGYXCI4SMZYE75EHQBR6PSTJG3M\""
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INITIAL_CONTRACTS="$SCRIPT_DIR/initial_contracts.json"
INITIAL_BATCH="$SCRIPT_DIR/initial_batch.json"

# Prints (does not execute) the commands to deploy a named registry.
deploy () {
    local name="$1"
    stellar registry deploy --contract-name "$name" --wasm-name registry -- --admin theahaco --manager "$ADMIN"
    stellar registry create-alias "$name"
}

# Prints (does not execute) the batch-register invocation for the given
# registry contract alias and the JSON array of [name, address, owner] tuples.
batch_register () {
    local alias="$1"
    local contracts_json="$2"
    stellar contract invoke --id "$alias" --source theahaco -- batch-register --contracts "$contracts_json"
}

# Per-project registries from initial_contracts.json:
# each top-level entry is {"<name>": [[name, address, owner], ...]}.
while IFS= read -r name; do
    CONTRACT_ID=$(stellar registry fetch-contract-id $name)
    echo WHEN contract_id = \'$CONTRACT_ID\' THEN \'$name\'

    # deploy "$name"
    # contracts=$(jq -c --arg k "$name" '.[] | select(has($k)) | .[$k]' "$INITIAL_CONTRACTS")
    # batch_register "$name" "$contracts"
    # stellar contract invoke --id "$name" -- process-batch --limit 10
done < <(jq -r '.[] | keys[0]' "$INITIAL_CONTRACTS")

# # Top-level contracts batch-registered into the root theahaco registry.
# batch_register theahaco "$(jq -c '.' "$INITIAL_BATCH")"
