#!/bin/sh
# Builds the browser version into web/dist (static files, any web server):
#
#   web/build.sh [--data DIR]     then   node web/serve.mjs  (or bun)
#
# --data DIR copies the original data files (e.g. KEEN4) to dist/data, so
# the page starts with them; without it the page asks for them (they stay
# in the browser). Don't publish dist/data with game data you may not
# distribute (the Keen 4 shareware may be).
set -e
DATA=
while [ $# -gt 0 ]; do
  case "$1" in
    --data) DATA=$(cd "$2" && pwd); shift 2 ;;
    *) echo "usage: $0 [--data DIR]" >&2; exit 1 ;;
  esac
done
cd "$(dirname "$0")"
wasm-pack build --release --target web --no-typescript --no-pack --out-dir pkg
rm -rf dist
mkdir -p dist/pkg
cp www/* dist/
cp ../icon.png dist/
cp pkg/keen_web.js pkg/keen_web_bg.wasm dist/pkg/
if [ -n "$DATA" ]; then
  mkdir -p dist/data
  list=
  for f in "$DATA"/*; do
    n=$(basename "$f")
    u=$(echo "$n" | tr '[:lower:]' '[:upper:]')
    case "$u" in
      CONFIG.*|SAVEGAM*|DEMO*) continue ;;
      *.CK4|*.CK5|*.CK6|*.EXE) cp "$f" dist/data/ && list="$list${list:+,}\"$n\"" ;;
    esac
  done
  echo "[$list]" > dist/data/files.json
  echo "data files: [$list]"
fi
echo "built web/dist ($(du -h dist/pkg/keen_web_bg.wasm | cut -f1) wasm)"
