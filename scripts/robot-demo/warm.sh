#!/usr/bin/env bash
# Runner B — warm phase.
#
# A NEW isolated sandbox (fresh worktree of the exact base revision, fresh
# empty outputs). Applies the reviewed candidate patch, rebuilds, and runs
# the protected checks anew. With IB present, the build reuses the
# parent-warmed cache namespace; locally this is a labeled native build.
set -euo pipefail
cd "$(dirname "$0")/../.."
ROOT="$(pwd)"
# shellcheck disable=SC1091
[ -f .env.local ] && { set -a; . ./.env.local; set +a; }

RUN_ID="${1:-warm-$(date +%s)}"
BACKEND="${ROBOT_DEMO_BACKEND:-mock}"
PATCH="${ROBOT_DEMO_PATCH_FILE:-demo/fallback-patch.diff}"
B=.runners/b
ts_ms() { python3 -c 'import time;print(int(time.time()*1000))'; }

echo "== [runner B] create isolated sandbox (fresh worktree + fresh outputs) =="
git worktree add --force "$B/src" HEAD >/dev/null
mkdir -p "$B/target" "$B/ib-cache"

echo "== [runner B] apply reviewed candidate patch: $PATCH =="
git -C "$B/src" apply --check "$ROOT/$PATCH"
git -C "$B/src" apply "$ROOT/$PATCH"
echo "patch applied to exact base $(git rev-parse --short HEAD)"

echo "== [runner B] warm build (fixed candidate) =="
START=$(ts_ms)
(
  cd "$B/src/rust"
  export CARGO_TARGET_DIR="$ROOT/$B/target"
  if command -v ib_console >/dev/null 2>&1; then
    echo "building via Incredibuild (parent-warmed cache namespace)"
    ib_console cargo build --locked
    ib_console cargo test -p robot-safety-gate -p swf-app --locked --no-run
  else
    echo "IB NOT PRESENT — native cargo build (labeled; no acceleration claimed)"
    cargo build --locked
    cargo test -p robot-safety-gate -p swf-app --locked --no-run
  fi
)
END=$(ts_ms)
mkdir -p "evidence/$RUN_ID"
cat >> "evidence/$RUN_ID/build-metrics.jsonl" <<EOF
{"phase":"warm","runner":"B","wall_ms":$((END-START)),"cache_namespace":"$B/ib-cache","patch":"$PATCH","ib":$(command -v ib_console >/dev/null 2>&1 && echo true || echo false)}
EOF

echo "== [runner B] protected contract suite on the fixed candidate =="
(
  cd "$B/src/rust"
  CARGO_TARGET_DIR="$ROOT/$B/target" cargo test -p robot-safety-gate --locked
)

echo "== [runner B] coverage matrix on the actual built executable =="
ROBOT_DEMO_ROOT="$ROOT/$B/src" ROBOT_DEMO_PYTHON="${ROBOT_DEMO_PYTHON:-python3}" \
  ROBOT_DEMO_PATCH="$PATCH" \
  "$B/target/debug/swf-cli" robot-demo matrix --backend "$BACKEND" --run-id "$RUN_ID"

echo "== [runner B] export run evidence before sandbox teardown =="
mkdir -p "evidence/$RUN_ID"
cp -r "$B/src/evidence/$RUN_ID/." "evidence/$RUN_ID/" 2>/dev/null || true

echo "== [runner B] export artifact + evidence, then destroy sandbox =="
mkdir -p "evidence/$RUN_ID/artifact"
cp "$B/target/debug/swf-cli" "evidence/$RUN_ID/artifact/"
shasum -a 256 "$B/target/debug/swf-cli" > "evidence/$RUN_ID/artifact/swf-cli.sha256"
git worktree remove --force "$B/src"
rm -rf "$B"
echo "run-id: $RUN_ID"
