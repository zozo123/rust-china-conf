#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

PYTHON="${ROBOT_DEMO_PYTHON:-$ROOT/demo/robot-sim/.venv/bin/python3}"
OUTPUT="${1:-$ROOT/docs/assets/robot-${SCENARIO:-fresh_lift}.gif}"
RUN_ID="asset-${SCENARIO:-fresh_lift}-$(date +%s)-$$"

for command in cargo ffmpeg; do
  command -v "$command" >/dev/null || {
    echo "missing required command: $command" >&2
    exit 1
  }
done

[[ -x "$PYTHON" ]] || {
  echo "missing simulator Python: $PYTHON" >&2
  echo "create demo/robot-sim/.venv and install requirements-linux.txt" >&2
  exit 1
}

"$PYTHON" -c "import imageio, mujoco, robosuite" >/dev/null

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"; rm -rf "$ROOT/evidence/$RUN_ID"' EXIT

mkdir -p "$(dirname "$OUTPUT")"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/rust/target}"
if [[ "$CARGO_TARGET_DIR" != /* ]]; then CARGO_TARGET_DIR="$ROOT/$CARGO_TARGET_DIR"; fi
cargo build --manifest-path rust/Cargo.toml --locked

ROBOT_DEMO_PYTHON="$PYTHON" \
ROBOT_DEMO_VIDEO_DIR="$TMP" \
  "$CARGO_TARGET_DIR/debug/swf-cli" robot-demo run \
    --scenario "${SCENARIO:-fresh_lift}" \
    --backend robosuite \
    --run-id "$RUN_ID" \
    --timeout-ms 30000

ffmpeg -y -i "$TMP/episode.mp4" \
  -filter_complex \
  "[0:v]setpts=1.5*PTS,fps=12,scale=512:-1:flags=lanczos,split[a][b];\
[a]palettegen=max_colors=192:stats_mode=diff[p];\
[b][p]paletteuse=dither=sierra2_4a:diff_mode=rectangle" \
  -loop 0 "$OUTPUT"

if command -v gifsicle >/dev/null; then
  gifsicle -O3 --colors 192 "$OUTPUT" -o "$TMP/optimized.gif"
  mv "$TMP/optimized.gif" "$OUTPUT"
fi

echo "wrote $OUTPUT"
