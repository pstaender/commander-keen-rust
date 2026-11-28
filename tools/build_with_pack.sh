#!/bin/sh
# Downloads the original Keen 4, 5 and 6 files from archive.org, builds
# keen4, keen5 and keen6 and packages each with a graphics pack, so they
# start with it (no --gfx-pack needed: the game picks up the gfx-pack
# directory next to the executable / inside the app bundle).
#
#   tools/build_with_pack.sh [--windows] [PACK_DIR] [OUT_DIR]
#
# Runs on macOS, Linux and Windows (Git Bash / MSYS2) and builds for the
# system it runs on: OUT_DIR/macos (Keen 4.app .. Keen 6.app),
# OUT_DIR/linux or OUT_DIR/windows (keen4..keen6 with data/ and gfx-pack/).
# --windows on macOS/Linux also cross-builds the Windows version (needs
# zig and cargo-zigbuild: brew install zig; cargo install cargo-zigbuild;
# the .exe files then have no icon). Linux can't be cross-built (ALSA and
# udev libraries): build it on Linux or with the GitHub workflow
# .github/workflows/build-pack.yml.
#
# PACK_DIR defaults to packs/keen-laptop, OUT_DIR to dist/<pack name>.
# Downloads are kept in downloads/ (git-ignored) and reused. The result
# contains the original game data: it must not be committed.
set -eu

cd "$(dirname "$0")/.."
cross_windows=0
if [ "${1:-}" = --windows ]; then cross_windows=1; shift; fi
pack=${1:-packs/keen-laptop}
name=$(basename "$pack")
out=${2:-dist/$name}
dl=downloads

[ -f "$pack/pack.txt" ] || { echo "error: $pack/pack.txt not found" >&2; exit 1; }

case "$(uname -s)" in
  Darwin) host=macos ;;
  Linux) host=linux ;;
  MINGW*|MSYS*|CYGWIN*) host=windows ;;
  *) echo "error: unknown system $(uname -s)" >&2; exit 1 ;;
esac
[ $host = windows ] && cross_windows=0

# --- download ---------------------------------------------------------------
mkdir -p "$dl/zips"
fetch() { # url file
  if [ ! -s "$dl/$2" ]; then
    echo "downloading $2"
    curl -sSfL --retry 3 -o "$dl/$2.part" "$1"
    mv "$dl/$2.part" "$dl/$2"
  fi
}
fetch https://archive.org/download/msdos_Commander_Keen_4_-_Secret_of_the_Oracle_1991/Commander_Keen_4_-_Secret_of_the_Oracle_1991.zip zips/keen4.zip
fetch https://archive.org/download/keen5/keen5.zip zips/keen5.zip
fetch https://archive.org/download/msdos_Commander_Keen_6_-_Aliens_Ate_My_Baby_Sitter_1991/Commander_Keen_6_-_Aliens_Ate_My_Baby_Sitter_1991.zip zips/keen6.zip

# GeneralUser GS SoundFont for the MIDI music (as in the release builds)
sf_base=https://raw.githubusercontent.com/mrbumpy409/GeneralUser-GS/684543d5e5efaef08d02be50dcda8d552478fa60
fetch "$sf_base/GeneralUser-GS.sf2" GeneralUser-GS.sf2
fetch "$sf_base/documentation/LICENSE.txt" GeneralUser-GS-LICENSE.txt
if command -v sha256sum >/dev/null; then sha() { sha256sum "$1" | cut -d' ' -f1; }
else sha() { shasum -a 256 "$1" | cut -d' ' -f1; }; fi
[ "$(sha "$dl/GeneralUser-GS.sf2")" = 9575028c7a1f589f5770fccc8cff2734566af40cd26ed836944e9a5152688cfe ] ||
  { echo "error: SoundFont checksum mismatch" >&2; exit 1; }

# --- unpack into downloads/KEEN4, KEEN5, KEEN6 --------------------------------
unpack() { # zip dir (unzip is missing in some Git Bash installations)
  if command -v unzip >/dev/null; then unzip -q -o "$1" -d "$2"
  elif command -v 7z >/dev/null; then 7z x -y -o"$2" "$1" >/dev/null
  else python3 -I -m zipfile -e "$1" "$2"; fi
}
for n in 4 5 6; do
  dst="$dl/KEEN$n"
  [ -f "$dst/EGAGRAPH.CK$n" ] && continue
  tmp="$dl/unpack$n"
  rm -rf "$tmp" && mkdir -p "$tmp"
  unpack "$dl/zips/keen$n.zip" "$tmp"
  # some archives hold another zip with the game
  if [ -z "$(find "$tmp" -iname "EGAGRAPH.CK$n" | head -n1)" ]; then
    find "$tmp" -iname '*.zip' | while read -r z; do unpack "$z" "$tmp/inner"; done
  fi
  egagraph=$(find "$tmp" -iname "EGAGRAPH.CK$n" | head -n1)
  [ -n "$egagraph" ] || { echo "error: no EGAGRAPH.CK$n in keen$n.zip" >&2; exit 1; }
  src=$(dirname "$egagraph")
  mkdir -p "$dst"
  # only the files the port reads, with upper-case names
  for f in "$src"/*; do
    [ -f "$f" ] || continue
    base=$(basename "$f" | tr '[:lower:]' '[:upper:]')
    case "$base" in
      AUDIO.CK$n|EGAGRAPH.CK$n|GAMEMAPS.CK$n|KEEN$n*.EXE) cp "$f" "$dst/$base" ;;
    esac
  done
  rm -rf "$tmp"
  echo "KEEN$n: $(ls "$dst" | tr '\n' ' ')"
done

# --- build ------------------------------------------------------------------
exe=""
if [ $host = macos ]; then
  rustup target add aarch64-apple-darwin x86_64-apple-darwin >/dev/null 2>&1 || true
  if cargo build --release -p keen --target aarch64-apple-darwin &&
     cargo build --release -p keen --target x86_64-apple-darwin; then
    mkdir -p target/universal
    for n in 4 5 6; do
      lipo -create -output "target/universal/keen$n" \
        "target/aarch64-apple-darwin/release/keen$n" \
        "target/x86_64-apple-darwin/release/keen$n"
    done
    bin=target/universal
  else
    echo "universal build failed, building for this Mac only"
    cargo build --release -p keen
    bin=target/release
  fi
else
  cargo build --release -p keen
  bin=target/release
  [ $host = windows ] && exe=.exe
fi
if [ $cross_windows = 1 ]; then
  command -v cargo-zigbuild >/dev/null ||
    { echo "error: --windows needs cargo-zigbuild (and zig)" >&2; exit 1; }
  rustup target add x86_64-pc-windows-gnu >/dev/null 2>&1 || true
  cargo zigbuild --release -p keen --target x86_64-pc-windows-gnu
fi

# --- check: each episode replays its first demo with the pack -----------------
for n in 4 5 6; do
  "$bin/keen$n$exe" --data "$dl/KEEN$n" --gfx-pack "$pack" --headless --nosound --demo 0 >/dev/null
  echo "keen$n: demo 0 ok with $pack"
done

# --- package ----------------------------------------------------------------
copy_pack() { # destination directory
  mkdir -p "$1"
  cp "$pack/pack.txt" "$1/"
  for d in "$pack"/*/; do
    d=${d%/}
    [ "$(basename "$d")" = source ] && continue # the pack's working files
    cp -R "$d" "$1/"
  done
  find "$1" -name .DS_Store -delete
}
package_dir() { # platform binary-dir exe-suffix: keen4..6 + data/ + gfx-pack/
  o="$out/$1"
  rm -rf "$o" && mkdir -p "$o/data"
  cp README.md "$o/"
  for n in 4 5 6; do
    cp "$2/keen$n$3" "$o/"
    cp -R "$dl/KEEN$n" "$o/data/"
  done
  cp "$dl/GeneralUser-GS.sf2" "$dl/GeneralUser-GS-LICENSE.txt" "$o/data/"
  copy_pack "$o/gfx-pack"
}
if [ $host = macos ]; then
  o="$out/macos"
  rm -rf "$o" && mkdir -p "$o"
  cp README.md "$o/"
  python3 -I tools/make_icns.py icon.png "$dl/keen.icns"
  for n in 4 5 6; do
    if [ $n = 4 ]; then # the SoundFont once, the other apps find it there
      tools/make_app.sh "Keen $n" "$bin/keen$n" "$dl/keen.icns" "$o" "$dl/KEEN$n" \
        "$dl/GeneralUser-GS.sf2" "$dl/GeneralUser-GS-LICENSE.txt"
    else
      tools/make_app.sh "Keen $n" "$bin/keen$n" "$dl/keen.icns" "$o" "$dl/KEEN$n"
    fi
    copy_pack "$o/Keen $n.app/Contents/Resources/gfx-pack"
    # sign again: the pack was added after make_app.sh signed the bundle
    codesign --force --deep -s - "$o/Keen $n.app" 2>/dev/null || true
  done
  check="$o/Keen 4.app/Contents/MacOS/keen4"
else
  package_dir $host "$bin" "$exe"
  check="$out/$host/keen4$exe"
fi
[ $cross_windows = 1 ] && package_dir windows target/x86_64-pc-windows-gnu/release .exe

# the packaged game must start without --data / --gfx-pack
check=$(cd "$(dirname "$check")" && pwd)/$(basename "$check")
(cd "${TMPDIR:-/tmp}" && "$check" --headless --nosound --demo 0 >/dev/null)
echo "done: $(ls -d "$out"/*/ | tr '\n' ' ')"
