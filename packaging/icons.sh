#!/usr/bin/env bash
# Regenerate every app icon from assets/app-icon/letra.svg.
#
# Needs: resvg (brew install resvg / cargo install resvg). On macOS, iconutil also writes the
# .icns. The outputs are committed, so builds and packaging never need these tools.
#
#   packaging/icons.sh
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIR="$ROOT/assets/app-icon"
SVG="$DIR/letra.svg"
ID="dev.danielmiguelt.Letra"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

command -v resvg >/dev/null || { echo "error: resvg not found (brew install resvg)" >&2; exit 1; }

# The artwork is a full-bleed 512 tile (rx=112). Windows and Linux use it as is. macOS icons sit on
# Apple's grid: an 824/1024 body with a transparent margin, so widen the viewBox to 512*1024/824.
MAC="$TMP/mac.svg"
sed 's/viewBox="0 0 512 512"/viewBox="-62.136 -62.136 636.272 636.272"/' "$SVG" >"$MAC"
grep -q 'viewBox="-62.136' "$MAC" || { echo "error: expected viewBox=\"0 0 512 512\" in $SVG" >&2; exit 1; }

render() {
  local src="$1" size="$2" dst="$3"
  resvg -w "$size" -h "$size" "$src" "$dst" </dev/null
  return 0
}

render "$SVG" 1024 "$DIR/letra-1024.png"
# Runtime window/Dock icon on macOS (embedded by apps/letra); other platforms use hicolor 256.
render "$MAC" 512 "$DIR/letra-macos-512.png"

# Linux hicolor theme: the scalable icon sits on the Nodalix/Colloid app grid (224/256 tile,
# 16 px margin, 51.5 px radius, soft shadow). It is made by tools/nodalia-app-icon.py in
# nodalix-icon-theme and committed; every hicolor size is rendered from it.
LINUX="$DIR/hicolor/scalable/apps/$ID.svg"
for s in 16 24 32 48 64 128 256 512; do
  mkdir -p "$DIR/hicolor/${s}x${s}/apps"
  render "$LINUX" "$s" "$DIR/hicolor/${s}x${s}/apps/$ID.png"
done

# Windows .ico.
ICO_PNGS=()
for s in 16 20 24 32 40 48 64 128 256; do
  render "$SVG" "$s" "$TMP/ico-$s.png"
  ICO_PNGS+=("$TMP/ico-$s.png")
done
(cd "$ROOT" && cargo run -q -p xtask -- ico "$DIR/letra.ico" "${ICO_PNGS[@]}")

# macOS .icns.
if command -v iconutil >/dev/null; then
  SET="$TMP/letra.iconset"
  mkdir -p "$SET"
  for s in 16 32 128 256 512; do
    render "$MAC" "$s" "$SET/icon_${s}x${s}.png"
    render "$MAC" $((s * 2)) "$SET/icon_${s}x${s}@2x.png"
  done
  iconutil -c icns -o "$DIR/letra.icns" "$SET"
else
  echo "warning: iconutil not found (macOS only); letra.icns not regenerated" >&2
fi
echo "icons written to $DIR"
