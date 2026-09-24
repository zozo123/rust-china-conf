set -euo pipefail
ROOT=$HOME/rust-china-conf
BASE=$(cd $ROOT && git rev-parse HEAD)
PATCH=$ROOT/demo/fallback-patch.diff
LAB=/tmp/finalnw
cd $ROOT
for w in $(git worktree list --porcelain | awk '/^worktree/{print $2}' | grep "^$LAB" || true); do git worktree remove --force "$w" >/dev/null 2>&1 || true; done
rm -rf $LAB; mkdir -p $LAB
ts(){ date +%s%3N; }

echo "=== BASE $BASE  patch sha $(sha256sum $PATCH | cut -c1-16) ==="
echo "=== VARIANT A: one worktree, patch applied IN PLACE, persistent CARGO_TARGET_DIR ==="
git worktree add --detach $LAB/wsA $BASE >/dev/null
TA=$LAB/targetA; mkdir -p $TA
(cd $LAB/wsA/rust && cargo fetch --locked >/dev/null 2>&1)
s=$(ts); (cd $LAB/wsA/rust && CARGO_TARGET_DIR=$TA cargo build --workspace --locked >/dev/null 2>&1); f=$(ts)
echo "A-seed-cold $((f-s)) ms"
(cd $LAB/wsA/rust && CARGO_TARGET_DIR=$TA cargo build --workspace --locked >/dev/null 2>&1)
for i in 1 2 3 4 5; do
  s=$(ts); (cd $LAB/wsA/rust && CARGO_TARGET_DIR=$TA cargo build --workspace --locked >/dev/null 2>&1); f=$(ts)
  echo "A-noop $i $((f-s)) ms"
  git -C $LAB/wsA apply -- $PATCH
  if [ $i = 1 ]; then
    s=$(ts); (cd $LAB/wsA/rust && CARGO_TARGET_DIR=$TA cargo build --workspace --locked 2> $LAB/cargo-apply-1.txt >/dev/null); f=$(ts)
  else
    s=$(ts); (cd $LAB/wsA/rust && CARGO_TARGET_DIR=$TA cargo build --workspace --locked >/dev/null 2>&1); f=$(ts)
  fi
  echo "A-apply $i $((f-s)) ms"
  grep -c "SEEDED REGRESSION" $LAB/wsA/rust/crates/robot-safety-gate/src/lib.rs >/dev/null && { echo "PATCH NOT APPLIED"; exit 9; } || true
  git -C $LAB/wsA checkout -- .
  s=$(ts); (cd $LAB/wsA/rust && CARGO_TARGET_DIR=$TA cargo build --workspace --locked >/dev/null 2>&1); f=$(ts)
  echo "A-revert $i $((f-s)) ms"
done
echo "--- crates compiled in A-apply rep1 ---"; grep -E "^\s+Compiling" $LAB/cargo-apply-1.txt || true

echo "=== VARIANT B: FRESH worktree per candidate (new path each time), ONE shared persistent target ==="
TB=$LAB/targetB; mkdir -p $TB
git worktree add --detach $LAB/wsB-parent $BASE >/dev/null
(cd $LAB/wsB-parent/rust && cargo fetch --locked >/dev/null 2>&1)
s=$(ts); (cd $LAB/wsB-parent/rust && CARGO_TARGET_DIR=$TB cargo build --workspace --locked >/dev/null 2>&1); f=$(ts)
echo "B-seed-cold $((f-s)) ms"
for i in 1 2 3 4 5; do
  W=$LAB/wsB-cand-$i
  git worktree add --detach $W $BASE >/dev/null
  git -C $W apply -- $PATCH
  (cd $LAB/wsB-parent/rust && CARGO_TARGET_DIR=$TB cargo build --workspace --locked >/dev/null 2>&1)
  if [ $i = 1 ]; then
    s=$(ts); (cd $W/rust && CARGO_TARGET_DIR=$TB cargo build --workspace --locked 2> $LAB/cargo-B-1.txt >/dev/null); f=$(ts)
  else
    s=$(ts); (cd $W/rust && CARGO_TARGET_DIR=$TB cargo build --workspace --locked >/dev/null 2>&1); f=$(ts)
  fi
  echo "B-freshworktree-apply $i $((f-s)) ms"
  grep -c "SEEDED REGRESSION" $W/rust/crates/robot-safety-gate/src/lib.rs >/dev/null && { echo "PATCH NOT APPLIED"; exit 9; } || true
done
echo "--- crates compiled in B rep1 ---"; grep -E "^\s+Compiling" $LAB/cargo-B-1.txt || true
echo "=== DONE ==="
uptime
