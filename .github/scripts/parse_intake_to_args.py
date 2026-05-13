#!/usr/bin/env python3
"""Translate a parsed registry-intake form into stellar-registry-cli args.

Input via env vars:
  KIND          — `publish` | `register` | `deploy` | `admin` (the form kind)
  PARSED_JSON   — JSON emitted by `issue-ops/parser@v5` (the issue body fields)

Outputs to $GITHUB_OUTPUT:
  subcommand    — the stellar-registry-cli subcommand to invoke
  args          — shlex-joined string of flags. Safe to `eval` after env-var passthrough.
  network       — `testnet` | `mainnet` (echoed for the workflow's environment selector)
  download_url  — optional. If set, the workflow must `curl -L -o $download_dest <url>`
                  before invoking, and the args already reference $download_dest.
  download_dest — optional. Path the workflow should download `download_url` to.

This script is the single source of truth for kind→method→flag mapping. Edits here
go through normal PR review.
"""

from __future__ import annotations

import json
import os
import shlex
import sys
from typing import Any


WASM_DOWNLOAD_DEST = "/tmp/registry-intake.wasm"


def fail(msg: str) -> "Never":  # type: ignore[name-defined]
    print(f"::error::{msg}", file=sys.stderr)
    sys.exit(1)


def emit(name: str, value: str) -> None:
    out_path = os.environ.get("GITHUB_OUTPUT")
    line = f"{name}={value}"
    if out_path:
        with open(out_path, "a", encoding="utf-8") as fh:
            fh.write(line + "\n")
    print(line)


def get(form: dict[str, Any], key: str, *, required: bool = False) -> str:
    value = (form.get(key) or "").strip() if isinstance(form.get(key), str) else ""
    if required and not value:
        fail(f"missing required form field `{key}`")
    return value


def build_publish(form: dict[str, Any]) -> tuple[str, list[str], str | None, str | None]:
    method = get(form, "method", required=True)
    wasm_name = get(form, "wasm_name", required=True)
    version = get(form, "version", required=True)
    wasm_hash = get(form, "wasm_hash")
    wasm_url = get(form, "wasm_url")
    author = get(form, "author")

    if method == "publish_hash":
        if not wasm_hash:
            fail("`publish_hash` requires `wasm_hash`")
        args = [
            "--wasm-hash", wasm_hash,
            "--wasm-name", wasm_name,
            "--version", version,
        ]
        if author:
            args += ["--author", author]
        return "publish-hash", args, None, None

    if method == "publish":
        if not wasm_url:
            fail("`publish` requires `wasm_url` so the workflow can fetch the wasm bytes")
        args = [
            "--wasm", WASM_DOWNLOAD_DEST,
            "--wasm-name", wasm_name,
            "--binver", version,
        ]
        if author:
            args += ["--author", author]
        return "publish", args, wasm_url, WASM_DOWNLOAD_DEST

    fail(f"unknown publish method `{method}` (expected `publish_hash` or `publish`)")


def build_register(form: dict[str, Any]) -> tuple[str, list[str], None, None]:
    contract_name = get(form, "contract_name", required=True)
    contract_address = get(form, "contract_address", required=True)
    owner = get(form, "owner")

    args = [
        "--contract-name", contract_name,
        "--contract-address", contract_address,
    ]
    if owner:
        args += ["--owner", owner]
    return "register-contract", args, None, None


def build_deploy(form: dict[str, Any]) -> tuple[str, list[str], None, None]:
    wasm_name = get(form, "wasm_name", required=True)
    contract_name = get(form, "contract_name", required=True)
    admin = get(form, "admin", required=True)
    version = get(form, "version")
    deployer = get(form, "deployer")
    constructor_args = (form.get("constructor_args") or "").strip()

    args = [
        "--wasm-name", wasm_name,
        "--contract-name", contract_name,
    ]
    if version:
        args += ["--version", version]
    if deployer:
        args += ["--deployer", deployer]

    args.append("--")
    args.append(f"--admin={admin}")
    # Each non-blank line of constructor_args is treated as one extra `--key=value`
    # arg passed to `__constructor`. Submitters write them one per line.
    for line in constructor_args.splitlines():
        line = line.strip()
        if line:
            args.append(line)

    return "deploy", args, None, None


def build_admin(form: dict[str, Any]) -> tuple[str, list[str], None, None]:
    method = get(form, "method", required=True)
    contract_name = get(form, "contract_name", required=True)

    if method == "update_contract_owner":
        new_owner = get(form, "new_owner", required=True)
        return "update-contract-owner", [
            "--contract-name", contract_name,
            "--new-owner", new_owner,
        ], None, None

    if method == "update_contract_address":
        new_address = get(form, "new_address", required=True)
        return "update-contract-address", [
            "--contract-name", contract_name,
            "--new-address", new_address,
        ], None, None

    if method == "rename_contract":
        new_name = get(form, "new_name", required=True)
        return "rename-contract", [
            "--contract-name", contract_name,
            "--new-name", new_name,
        ], None, None

    if method == "upgrade_contract":
        wasm_name = get(form, "wasm_name", required=True)
        version = get(form, "version")
        args = [
            "--contract-name", contract_name,
            "--wasm-name", wasm_name,
        ]
        if version:
            args += ["--version", version]
        return "upgrade", args, None, None

    fail(f"unknown admin method `{method}`")


def main() -> int:
    kind = (os.environ.get("KIND") or "").strip()
    raw = (os.environ.get("PARSED_JSON") or "").strip()
    if not kind:
        fail("KIND env var is required")
    if not raw:
        fail("PARSED_JSON env var is required")

    try:
        form = json.loads(raw)
    except json.JSONDecodeError as exc:
        fail(f"PARSED_JSON is not valid JSON: {exc}")

    if not isinstance(form, dict):
        fail("PARSED_JSON must decode to an object")

    network = get(form, "network", required=True)
    if network not in {"testnet", "mainnet"}:
        fail(f"network must be `testnet` or `mainnet`, got `{network}`")

    builders = {
        "publish": build_publish,
        "register": build_register,
        "deploy": build_deploy,
        "admin": build_admin,
    }
    if kind not in builders:
        fail(f"unknown kind `{kind}` (expected one of {sorted(builders)})")

    subcommand, args, download_url, download_dest = builders[kind](form)

    emit("subcommand", subcommand)
    emit("args", shlex.join(args))
    emit("network", network)
    emit("download_url", download_url or "")
    emit("download_dest", download_dest or "")
    return 0


if __name__ == "__main__":
    sys.exit(main())
