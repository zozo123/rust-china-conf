set -euo pipefail
ROOT=$HOME/rust-china-conf
BASE=$(cd $ROOT && git rev-parse HEAD)
PATCH=$ROOT/demo/fallback-patch.diff
LAB=/tmp/stalecheck
cd $ROOT
for w in $(git worktree list --porcelain | awk '/^worktree/{print $2}' | grep "^$LAB" || true); do git worktree remove --force "$w" >/dev/null 2>&1 || true; done
rm -rf $LAB; mkdir -p $LAB
TG=$LAB/target; mkdir -p $TG
ts(){ date +%s%3N; }
git worktree add --detach $LAB/wsP $BASE >/dev/null
git worktree add --detach $LAB/wsC $BASE >/dev/null
git -C $LAB/wsC apply -- $PATCH
# make the CANDIDATE's changed file OLDER than the parent's artifacts
touch -d '2020-01-01 00:00:00' $LAB/wsC/rust/crates/robot-safety-gate/src/lib.rs
(cd $LAB/wsP/rust && cargo fetch --locked >/dev/null 2>&1)
(cd $LAB/wsP/rust && CARGO_TARGET_DIR=$TG cargo build --workspace --locked >/dev/null 2>&1)
P=$(sha256sum $TG/debug/librobot_safety_gate.rlib | cut -c1-16)
echo "parent rlib   $P"
s=$(ts); (cd $LAB/wsC/rust && CARGO_TARGET_DIR=$TG cargo build --workspace --locked 2>$LAB/c1.txt >/dev/null); f=$(ts)
C=$(sha256sum $TG/debug/librobot_safety_gate.rlib | cut -c1-16)
echo "candidate (old mtime) build $((f-s)) ms -> rlib $C  compiled: $(grep -c Compiling $LAB/c1.txt || true)"
[ "$P" = "$C" ] && echo "RESULT: STALE ARTIFACT REUSED (byte-identical to parent)" || echo "RESULT: rebuilt correctly"
touch $LAB/wsC/rust/crates/robot-safety-gate/src/lib.rs
s=$(ts); (cd $LAB/wsC/rust && CARGO_TARGET_DIR=$TG cargo build --workspace --locked >/dev/null 2>&1); f=$(ts)
D=$(sha256sum $TG/debug/librobot_safety_gate.rlib | cut -c1-16)
echo "after touch: $((f-s)) ms -> rlib $D"
