#!/usr/bin/env bash
# Usage: validate.sh <run-id> [--scenario <name>]
# Default: require every protected matrix scenario exactly once.
set -euo pipefail
cd "$(dirname "$0")/../.."
RUN_ID="${1:?usage: validate.sh <run-id> [--scenario <name>]}"
shift
[[ "$RUN_ID" =~ ^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$ ]] || { echo "invalid run id" >&2; exit 1; }
"${ROBOT_DEMO_PYTHON:-python3}" demo/robot-sim/acceptance/verify_run.py "$(pwd)/evidence/$RUN_ID" "$@"
