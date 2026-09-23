#!/usr/bin/env bash
# Validate: protected contract suite + protected evidence verifier.
# Usage: validate.sh <run-id>
set -euo pipefail
cd "$(dirname "$0")/../.."
ROOT="$(pwd)"

RUN_ID="${1:?usage: validate.sh <run-id>}"

echo "== protected contract suite (current checkout) =="
( cd rust && cargo test -p robot-safety-gate --locked )

echo "== protected evidence verifier (run-id: $RUN_ID) =="
VERIFIER=demo/robot-sim/acceptance/verify_run.py
# The verifier runs from the protected in-repo copy — never from anything
# an agent run may have placed inside the evidence dir.
python3 "$VERIFIER" "$ROOT/evidence/$RUN_ID"
