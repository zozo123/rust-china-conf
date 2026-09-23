#!/usr/bin/env bash
# Runner A — cold phase.
#
# Starts an isolated sandbox (a fresh git worktree of the pinned seeded
# revision, a fresh empty CARGO_TARGET_DIR, and an empty compilation-cache
# namespace), builds the seeded revision, and runs the failing acceptance
# case. Exports the failure context packet for the agent.
#
# When a remote islo sandbox provider is configured (ISLO_SANDBOX_KEY in
# .env.local + provider CLI), this same phase dispatches there instead;
# the local path below is the self-contained equivalent.
set -euo pipefail
cd "$(dirname "$0")/../.."
ROOT="$(pwd)"
# shellcheck disable=SC1091
[ -f .env.local ] && { set -a; . ./.env.local; set +a; }

RUN_ID="${1:-cold-$(date +%s)}"
BACKEND="${ROBOT_DEMO_BACKEND:-mock}"
A=.runners/a
ts_ms() { python3 -c 'import time;print(int(time.time()*1000))'; }

echo "== [runner A] create isolated sandbox (worktree @ $(git rev-parse --short HEAD), fresh outputs) =="
git worktree add --force "$A/src" HEAD >/dev/null
mkdir -p "$A/target" "$A/ib-cache"   # empty compilation outputs + cache namespace

echo "== [runner A] cold build (seeded revision) =="
START=$(ts_ms)
(
  cd "$A/src/rust"
  export CARGO_TARGET_DIR="$ROOT/$A/target"
  if command -v ib_console >/dev/null 2>&1; then
    echo "building via Incredibuild (cache namespace: $A/ib-cache)"
    ib_console cargo build --locked
    ib_console cargo test -p robot-safety-gate -p swf-app --locked --no-run
  else
    echo "IB NOT PRESENT — native cargo build (labeled baseline)"
    cargo build --locked
    cargo test -p robot-safety-gate -p swf-app --locked --no-run
  fi
)
END=$(ts_ms)
mkdir -p "evidence/$RUN_ID"
cat >> "evidence/$RUN_ID/build-metrics.jsonl" <<EOF
{"phase":"cold","runner":"A","wall_ms":$((END-START)),"cache_namespace":"$A/ib-cache","ib":$(command -v ib_console >/dev/null 2>&1 && echo true || echo false)}
EOF

echo "== [runner A] failing acceptance case (stale_600ms on seeded gate) =="
set +e
ROBOT_DEMO_ROOT="$ROOT/$A/src" ROBOT_DEMO_PYTHON="${ROBOT_DEMO_PYTHON:-python3}" \
  "$A/target/debug/swf-cli" robot-demo run --scenario stale_600ms --backend "$BACKEND" --run-id "$RUN_ID"
set -e

echo "== [runner A] export run evidence before sandbox teardown =="
mkdir -p "evidence/$RUN_ID"
cp -r "$A/src/evidence/$RUN_ID/." "evidence/$RUN_ID/" 2>/dev/null || true

echo "== [runner A] export agent context packet =="
mkdir -p "evidence/$RUN_ID/agent-context"
cp "$A/src/rust/crates/robot-safety-gate/src/lib.rs" "evidence/$RUN_ID/agent-context/"
cat > "evidence/$RUN_ID/agent-context/work-order.md" <<'EOF'
Fix the seeded stale-observation regression in robot-safety-gate. At dispatch,
observations older than 250 ms must return StalePerception. Preserve the
simulated emergency-stop precedence and timestamp validation. Preserve
boundary behavior at 250 and 251 ms. Do not edit simulator code, fixtures,
the threshold, acceptance checks, runner scripts or evidence generation.
Return the patch and relevant test output.
EOF
git rev-parse HEAD > "evidence/$RUN_ID/agent-context/base-revision.txt"

echo "== [runner A] evidence exported to evidence/$RUN_ID — destroying sandbox =="
git worktree remove --force "$A/src"
rm -rf "$A"
echo "The runner is gone. The change, the investigation, and the reusable compilation work survive."
echo "run-id: $RUN_ID"
