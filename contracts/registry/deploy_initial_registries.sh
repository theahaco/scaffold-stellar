#!/usr/bin/env bash
set -euo pipefail

DRY_RUN=0
for arg in "$@"; do
    case "$arg" in
        --dry-run|-n)
            DRY_RUN=1
            ;;
        -h|--help)
            echo "Usage: $0 [--dry-run|-n]"
            exit 0
            ;;
        *)
            echo "Unknown option: $arg" >&2
            echo "Usage: $0 [--dry-run|-n]" >&2
            exit 1
            ;;
    esac
done

run() {
    if [ "$DRY_RUN" -eq 1 ]; then
        printf '[dry-run] '
        printf '%q ' "$@"
        printf '\n'
    else
        "$@"
    fi
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PATH=$SCRIPT_DIR/../../target/debug:$PATH

ROOT_REGISTRY=$(stellar registry fetch-contract-id registry)

ADMIN=theahaco
MANAGER="\"$(stellar keys public-key $ADMIN)\""
INITIAL_CONTRACTS="$SCRIPT_DIR/initial_contracts.json"

# Deploys a named registry if it isn't already registered. Always ensures the
# local stellar alias exists.
deploy () {
    local name="$1"
    if existing_id=$(stellar registry fetch-contract-id "$name" 2>/dev/null) && [ -n "$existing_id" ]; then
        echo "Contract '$name' already registered (id: $existing_id); skipping deploy"
    else
        run stellar registry deploy --contract-name "$name" --wasm-name registry -- \
                                    --admin $ADMIN \
                                    --manager "$MANAGER" \
                                    --root "\"$ROOT_REGISTRY\""
    fi
    run stellar registry create-alias "$name" --force
}

# Prints (does not execute) the batch-register invocation for the given
# registry contract alias and the JSON array of [name, address, owner] tuples.
batch_register () {
    local alias="$1"
    local contracts_json="$2"
    run stellar contract invoke --id "$alias" --source theahaco -- batch-register --contracts "$contracts_json"
    run stellar contract invoke --id "$alias" --source theahaco -- process_batch --limit 10
}

# Per-project registries from initial_contracts.json:
# each top-level entry is {"<name>": [[name, address, owner], ...]}.
while IFS= read -r name; do
    deploy "$name"
    contracts=$(jq -c --arg k "$name" '.[] | select(has($k)) | .[$k]' "$INITIAL_CONTRACTS")
    if [ "$(jq 'length' <<<"$contracts")" -eq 0 ]; then
        echo "No contracts for '$name'; skipping batch-register"
        continue
    fi
    batch_register "$name" "$contracts"
done < <(jq -r '.[] | keys[0]' "$INITIAL_CONTRACTS")
