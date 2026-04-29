#!/usr/bin/env bash
set -e

PATH=./target/bin:$PATH
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$SCRIPT_DIR/../.."

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

# sha256 -s verified
#

registry_version() {
     awk -F'"' '/^version[[:space:]]*=/ { print $2; exit }' \
         "$REPO_ROOT/contracts/registry/Cargo.toml"
}
VERSION=v$(registry_version)
WASM_URL="https://github.com/theahaco/scaffold-stellar/releases/download/registry-$VERSION/registry_$VERSION.wasm"
WASM_PATH="$REPO_ROOT/target/stellar/registry_$VERSION.wasm"


if [ "$DRY_RUN" -eq 1 ]; then
    echo "[dry-run] curl -L $WASM_URL > $WASM_PATH"
else
    curl -L "$WASM_URL" > "$WASM_PATH"
fi


SALT=$(shasum -a 256 < "$REPO_DIR/crates/stellar-registry-build/.salt" | awk '{print $1}')

ADMIN=theahaco
ADDRESS=GAMPJROHOAW662FINQ4XQOY2ULX5IEGYXCI4SMZYE75EHQBR6PSTJG3M

run stellar contract deploy --alias registry \
                        --wasm "$WASM_PATH" \
                        --source "$ADMIN" \
                        --salt "$SALT" \
                        -- \
                        --admin "$ADMIN" \
                        --manager "\"$ADDRESS\""

run just registry publish  --wasm "$WASM_PATH" \
                         --author "$ADMIN" \
                         --source "$ADMIN"
