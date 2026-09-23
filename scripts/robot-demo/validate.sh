#!/usr/bin/env bash
# Validate: protected contract suite + protected evidence verifier.
# Usage: validate.sh <run-id>
set -euo pipefail
cd "$(dirname "$0")/../.."
ROOT="$(pwd)"

RUN_ID="${1:?usage: validate.sh <run-id>}"

# NOTE: the contract suite runs inside the runner phases (cold.sh / warm.sh)
# against each candidate build. main intentionally carries the seeded
# regression, so testing the current checkout here would fail by design.
# This script validates the RUN EVIDENCE with the protected verifier.

echo "== protected evidence verifier (run-id: $RUN_ID) =="
VERIFIER=demo/robot-sim/acceptance/verify_run.py
# The verifier runs from the protected in-repo copy — never from anything
# an agent run may have placed inside the evidence dir.
python3 "$VERIFIER" "$ROOT/evidence/$RUN_ID"
