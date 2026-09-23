#!/usr/bin/env bash
# Runner B: fresh outputs at the supplied base, allowlisted candidate, protected checks.
# ROBOT_DEMO_BASE_REVISION must be Runner A's exported revision for a paired run.
# Incredibuild can be invoked if present; no configured cache or reuse is claimed.
set -euo pipefail
RUNNER=B PHASE=warm RUN_ID="${1:-warm-$(date +%s)}"
# shellcheck source=scripts/robot-demo/runner-common.sh
source "$(dirname "$0")/runner-common.sh"
PATCH="${ROBOT_DEMO_PATCH_FILE:-$ROOT/demo/fallback-patch.diff}"
if [[ "$PATCH" != /* ]]; then PATCH="$ROOT/$PATCH"; fi
"$PYTHON" "$ROOT/scripts/robot-demo/check-patch.py" "$PATCH"
git -C "$SRC" apply --check -- "$PATCH"
git -C "$SRC" apply -- "$PATCH"
echo "candidate applied to exact base $BASE"
cp "$PATCH" "$EVIDENCE/candidate.patch"
build_candidate
export_artifact
(
  cd "$SRC/rust"
  cargo test -p robot-safety-gate --locked
)
ROBOT_DEMO_ROOT="$SRC" ROBOT_DEMO_PATCH="$PATCH" \
  "$TARGET/debug/swf-cli" robot-demo matrix --backend "$BACKEND" --run-id "$RUN_ID"
echo "run-id: $RUN_ID (runner removed on exit; validate exported evidence next)"
