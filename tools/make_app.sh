#!/bin/sh
# Wraps a binary into a macOS app bundle:
#   tools/make_app.sh "Keen 4" target/release/keen4 keen.icns OUTDIR [DATADIR]
# DATADIR (e.g. KEEN4) is copied to Contents/Resources/data/.
set -e
name=$1 bin=$2 icns=$3 out=$4 data=$5
exe=$(basename "$bin")
app="$out/$name.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$bin" "$app/Contents/MacOS/$exe"
cp "$icns" "$app/Contents/Resources/keen.icns"
if [ -n "$data" ]; then
  mkdir -p "$app/Contents/Resources/data"
  cp -R "$data" "$app/Contents/Resources/data/"
fi
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)
cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>$name</string>
  <key>CFBundleDisplayName</key><string>$name</string>
  <key>CFBundleExecutable</key><string>$exe</string>
  <key>CFBundleIconFile</key><string>keen.icns</string>
  <key>CFBundleIdentifier</key><string>io.github.commander-keen-rust.$exe</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST
# ad-hoc signature (Apple Silicon refuses unsigned code)
codesign --force --deep -s - "$app" 2>/dev/null || true
