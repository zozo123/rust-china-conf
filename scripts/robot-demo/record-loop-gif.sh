#!/usr/bin/env bash
# Build the loop GIF by screenshotting docs/demo/loop.html frame by frame.
# The page is frame-addressable (?frame=N), so output is deterministic.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

OUTPUT="${1:-$ROOT/docs/assets/validation-loop.gif}"
PAGE="${LOOP_PAGE:-$ROOT/docs/demo/loop.html}"
FRAMES=16          # must match the F array length in loop.html
WIDTH=880
HEIGHT=585

CHROME="${CHROME:-/Applications/Google Chrome.app/Contents/MacOS/Google Chrome}"
[[ -x "$CHROME" ]] || CHROME="$(command -v chromium || command -v google-chrome || true)"
[[ -n "$CHROME" && -x "$CHROME" ]] || { echo "no Chrome/Chromium found; set CHROME" >&2; exit 1; }
command -v ffmpeg >/dev/null || { echo "missing ffmpeg" >&2; exit 1; }

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$(dirname "$OUTPUT")"

for i in $(seq 0 $((FRAMES - 1))); do
  "$CHROME" --headless --disable-gpu --hide-scrollbars \
    --window-size="$WIDTH,$HEIGHT" \
    --virtual-time-budget=1200 \
    --screenshot="$TMP/$(printf '%03d' "$i").png" \
    "file://$PAGE?frame=$i" >/dev/null 2>&1
done

# Hold the closing frame so the proof stays on screen before looping.
last="$TMP/$(printf '%03d' $((FRAMES - 1))).png"
for j in $(seq "$FRAMES" $((FRAMES + 5))); do
  cp "$last" "$TMP/$(printf '%03d' "$j").png"
done

ffmpeg -y -framerate 1.15 -pattern_type glob -i "$TMP/*.png" \
  -filter_complex "fps=8,scale=${WIDTH}:-1:flags=lanczos,split[a][b];\
[a]palettegen=max_colors=128:stats_mode=full[p];\
[b][p]paletteuse=dither=bayer:bayer_scale=4" \
  -loop 0 "$OUTPUT" >/dev/null 2>&1

# H.264 + poster for the website: 19 s of motion needs a pause control.
ffmpeg -y -framerate 1.15 -pattern_type glob -i "$TMP/*.png" \
  -vf "scale=${WIDTH}:586:flags=lanczos,format=yuv420p" \
  -c:v libx264 -preset slow -crf 20 -movflags +faststart -an \
  "${OUTPUT%.gif}.mp4" >/dev/null 2>&1
cp "$TMP/000.png" "${OUTPUT%.gif}-poster.png"

if command -v gifsicle >/dev/null; then
  gifsicle -O3 --colors 128 "$OUTPUT" -o "$TMP/opt.gif" 2>/dev/null
  mv "$TMP/opt.gif" "$OUTPUT"
fi

echo "wrote $OUTPUT ($(du -h "$OUTPUT" | cut -f1))"
echo "wrote ${OUTPUT%.gif}.mp4 ($(du -h "${OUTPUT%.gif}.mp4" | cut -f1))"
echo "wrote ${OUTPUT%.gif}-poster.png"
