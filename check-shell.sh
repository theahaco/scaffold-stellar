#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

if ! command -v shellcheck >/dev/null 2>&1; then
    echo "error: shellcheck is not installed" >&2
    echo "install with: apt install shellcheck | brew install shellcheck" >&2
    exit 127
fi

mapfile -d '' scripts < <(
    find . \
        \( -path ./target -o -path ./node_modules -o -path '*/target' -o -path '*/node_modules' \) -prune \
        -o -type f -name '*.sh' -print0
)

if [ "${#scripts[@]}" -eq 0 ]; then
    echo "no shell scripts found"
    exit 0
fi

echo "checking ${#scripts[@]} script(s) with shellcheck $(shellcheck --version | awk '/^version:/ {print $2}')"

fail=0
for script in "${scripts[@]}"; do
    echo "-> $script"
    if ! shellcheck -x "$script"; then
        fail=1
    fi
done

if [ "$fail" -ne 0 ]; then
    echo "shellcheck found issues" >&2
    exit 1
fi

echo "all scripts passed"
