#!/usr/bin/env bash
# Full rehearsal: the six-minute stage arc, end to end.
#
#   preflight -> runner A (cold, seeded, failing case) -> agent patch
#   -> destroy A -> runner B (warm, fixed) -> protected validation
#
# Backend: ROBOT_DEMO_BACKEND=mock (default, runs anywhere) or robosuite
# (the real SIL path; requires demo/robot-sim/.venv per README).
set -euo pipefail
cd "$(dirname "$0")/../.."
BACKEND="${ROBOT_DEMO_BACKEND:-mock}"
RUN_ID="${1:-rehearsal-$(date +%s)}"

banner() { printf '\n\033[1m== [%s] %s ==\033[0m\n' "$(date +%M:%S)" "$1"; }

banner "0:00  preflight"
scripts/robot-demo/preflight.sh

banner "0:30  runner A: cold build + failing assertion (seeded revision)"
scripts/robot-demo/cold.sh "$RUN_ID"

banner "1:15  agent step: reviewed candidate patch"
echo "work order: evidence/$RUN_ID/agent-context/work-order.md"
echo "candidate:  demo/fallback-patch.diff"
sed -n '1,40p' demo/fallback-patch.diff

banner "2:30  runner B: warm build + protected checks (fixed candidate)"
scripts/robot-demo/warm.sh "$RUN_ID"

banner "4:40  protected validation of run evidence"
scripts/robot-demo/validate.sh "$RUN_ID"

banner "5:20  evidence summary"
echo "evidence/$RUN_ID:"
ls -1 "evidence/$RUN_ID"
echo
echo "Rehearsal arc complete (backend: $BACKEND). run-id: $RUN_ID"
