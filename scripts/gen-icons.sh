#!/usr/bin/env bash
# Regenerate platform icons from the SVG master.
#
# Requires: Node (for @tauri-apps/cli), rsvg-convert (librsvg) or ImageMagick
# for SVG -> PNG rasterization.

set -euo pipefail

cd "$(dirname "$0")/.."

ICON_SRC="src-tauri/icons/master.svg"
OUT_DIR="src-tauri/icons"
BASE_PNG="$OUT_DIR/icon.png"

if ! command -v rsvg-convert >/dev/null 2>&1; then
  if command -v magick >/dev/null 2>&1; then
    magick "$ICON_SRC" -resize 1024x1024 "$BASE_PNG"
  else
    echo "error: install librsvg (rsvg-convert) or ImageMagick (magick) first" >&2
    exit 1
  fi
else
  rsvg-convert -w 1024 -h 1024 "$ICON_SRC" -o "$BASE_PNG"
fi

npx --yes @tauri-apps/cli icon "$BASE_PNG" -o "$OUT_DIR"

echo "done: platform icons written to $OUT_DIR"
