#!/usr/bin/env python3
"""Render issue-ops/validator's `errors` JSON output into a friendly markdown comment.

Reads `COMMENT_ERRORS` from the environment (a JSON string emitted by
`issue-ops/validator@v4`'s `errors` output) and prints a markdown body to stdout.
The caller pipes that into `gh issue comment --body-file -`.
"""

from __future__ import annotations

import json
import os
import sys


def main() -> int:
    raw = os.environ.get("COMMENT_ERRORS", "").strip()
    if not raw:
        print(":x: Validation failed but no error details were emitted by the validator.")
        return 0

    try:
        errors = json.loads(raw)
    except json.JSONDecodeError:
        print(":x: Validation failed. Raw validator output:\n\n```\n" + raw + "\n```")
        return 0

    if not isinstance(errors, list) or not errors:
        print(":x: Validation failed but the error list was empty or malformed.")
        return 0

    lines = [
        ":x: **Validation failed.** Please edit the issue to fix the problems below — the form will be re-validated automatically.",
        "",
    ]
    for err in errors:
        if isinstance(err, dict):
            field = err.get("field") or err.get("name") or "(field)"
            msg = err.get("message") or err.get("error") or json.dumps(err)
            lines.append(f"- **`{field}`** — {msg}")
        else:
            lines.append(f"- {err}")
    lines.append("")
    lines.append(
        "Once you've edited the fields, the `issueops:validation-error` label will be removed and the issue will move to `registry-intake:in-review`."
    )

    print("\n".join(lines))
    return 0


if __name__ == "__main__":
    sys.exit(main())
