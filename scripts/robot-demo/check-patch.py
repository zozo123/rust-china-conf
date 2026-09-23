#!/usr/bin/env python3
"""Reject candidate patches outside the single permitted gate source file."""

import argparse
import re
import subprocess
import sys
from pathlib import Path

ALLOWED_PATH = "rust/crates/robot-safety-gate/src/lib.rs"


def validate_patch(path, root):
    patch = path.read_bytes()
    if re.search(rb"^(?:old mode|new mode|new file mode|deleted file mode|rename from|rename to|copy from|copy to) ", patch, re.M):
        raise ValueError("candidate must modify an existing regular source file; file/mode changes are forbidden")
    completed = subprocess.run(
        ["git", "apply", "--numstat", "-z", "--", str(path.resolve())],
        cwd=root, capture_output=True, check=True,
    )
    records = completed.stdout.rstrip(b"\0").split(b"\0")
    if not records or records == [b""]:
        raise ValueError("candidate patch is empty")
    for record in records:
        fields = record.split(b"\t", 2)
        if len(fields) != 3 or fields[2] != ALLOWED_PATH.encode():
            raise ValueError(f"candidate may only change {ALLOWED_PATH}")
        if not all(field.isdigit() for field in fields[:2]):
            raise ValueError("binary candidate patches are forbidden")
    return ALLOWED_PATH


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("patch", type=Path)
    args = parser.parse_args()
    try:
        changed = validate_patch(args.patch, Path(__file__).resolve().parents[2])
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        print(f"PATCH REJECTED: {error}", file=sys.stderr)
        return 1
    print(f"candidate allowlist passed: {changed}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
