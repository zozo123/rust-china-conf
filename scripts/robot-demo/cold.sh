#!/usr/bin/env bash
# Runner A: pinned seeded revision, fresh outputs, intentional stale failure.
set -euo pipefail
RUNNER=A PHASE=cold RUN_ID="${1:-cold-$(date +%s)}"
# shellcheck source=scripts/robot-demo/runner-common.sh
source "$(dirname "$0")/runner-common.sh"
build_candidate
export_artifact

echo "== runner A: seeded stale-observation episode =="
ROBOT_DEMO_ROOT="$SRC" "$TARGET/debug/swf-cli" robot-demo run \
  --scenario stale_600ms --backend "$BACKEND" --run-id "$RUN_ID"
# A failed command is an infrastructure error. The expected regression is a
# completed pickup despite stale perception, recorded in the result itself.
"$PYTHON" - "$SRC/evidence/$RUN_ID/scenario-results.json" <<'PY'
import json, sys
results = json.load(open(sys.argv[1], encoding="utf-8"))
if len(results) != 1 or results[0]["scenario"] != "stale_600ms" or results[0]["outcome"] != "cube_lifted" or not results[0]["success"] or results[0]["task_dispatches"] <= 0:
    raise SystemExit("seeded regression was not reproduced; refusing to export a misleading failure packet")
print("expected seed violation reproduced: stale perception still dispatched the pickup")
PY
mkdir -p "$EVIDENCE/agent-context"
cp "$SRC/rust/crates/robot-safety-gate/src/lib.rs" "$EVIDENCE/agent-context/"
cat > "$EVIDENCE/agent-context/work-order.md" <<'EOF'
Fix the seeded stale-observation regression in robot-safety-gate. At dispatch,
observations older than 250 ms must return StalePerception. Preserve the
simulated emergency-stop precedence and timestamp validation. Preserve
boundary behavior at 250 and 251 ms. Only rust/crates/robot-safety-gate/src/lib.rs
may be changed. Do not edit simulator code, fixtures, the threshold, acceptance
checks, runner scripts or evidence generation. Return the patch and test output.
EOF
printf '%s\n' "$BASE" > "$EVIDENCE/agent-context/base-revision.txt"
echo "run-id: $RUN_ID (seeded failure; runner removed on exit)"
