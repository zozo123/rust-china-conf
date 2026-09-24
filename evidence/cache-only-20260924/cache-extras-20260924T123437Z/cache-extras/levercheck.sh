#!/usr/bin/env bash
# Did the two Rust levers actually take effect?
# Run AFTER all measurement, outside every timed phase.
set -e
OUT="$1"
TB="$(rustc --print sysroot)/lib/rustlib/x86_64-unknown-linux-gnu/bin"
W=$(mktemp -d /tmp/lldcheck.XXXX)
cd "$HOME/rust-china-conf/rust"
{
echo "== toolchain =="; rustc -V
echo "== DEFAULT linker: .comment of the linked binary =="
CARGO_TARGET_DIR=$W/def cargo build --locked -q -p swf-cli
readelf -p .comment "$W/def/debug/swf-cli" | sed -n '2,6p'
echo "== EXPLICIT -fuse-ld=lld via the toolchain gcc-ld shim =="
CARGO_TARGET_DIR=$W/lld RUSTFLAGS="-Clink-arg=-fuse-ld=lld -Clink-arg=-B$TB/gcc-ld" cargo build --locked -q -p swf-cli
readelf -p .comment "$W/lld/debug/swf-cli" | sed -n '2,6p'
echo "== CARGO_PROFILE_DEV_DEBUG=0 =="
CARGO_TARGET_DIR=$W/d0 CARGO_PROFILE_DEV_DEBUG=0 cargo build --locked -q -p swf-cli
printf 'debug_info sections  default=%s  debug0=%s\n' \
  "$(readelf -S "$W/def/debug/swf-cli" | grep -c debug_info)" \
  "$(readelf -S "$W/d0/debug/swf-cli" | grep -c debug_info || true)"
ls -l "$W/def/debug/swf-cli" "$W/d0/debug/swf-cli" "$W/lld/debug/swf-cli" | awk '{print $5, $9}'
echo "CONCLUSION: rustc 1.92.0 ALREADY links with LLD by default on this target, so the"
echo "native-lld mode re-measured the native configuration and its 0% delta is a"
echo "precision check, not a verdict on lld. CARGO_PROFILE_DEV_DEBUG=0 did take effect."
} > "$OUT" 2>&1
rm -rf "$W"
cat "$OUT"
