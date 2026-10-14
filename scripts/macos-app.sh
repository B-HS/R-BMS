#!/usr/bin/env bash
set -euo pipefail

BINARY="${1:-}"
VERSION="${2:-}"
OUT="${3:-}"
if [ -z "$BINARY" ] || [ ! -f "$BINARY" ] || [ -z "$VERSION" ] || [ -z "$OUT" ]; then
    echo "usage: bash scripts/macos-app.sh <rbms-player binary> <version> <output folder>" >&2
    exit 2
fi

APP_NAME="R-BMS"
EXECUTABLE="rbms-player"
BUNDLE_ID="uk.hyuns.rbms"
MINIMUM_MACOS="11.0"
APP="$OUT/$APP_NAME.app"

if [ -e "$APP" ]; then
    echo "$APP already exists; give an empty output folder" >&2
    exit 1
fi
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BINARY" "$APP/Contents/MacOS/$EXECUTABLE"
chmod +x "$APP/Contents/MacOS/$EXECUTABLE"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleDisplayName</key>
    <string>$APP_NAME</string>
    <key>CFBundleExecutable</key>
    <string>$EXECUTABLE</string>
    <key>CFBundleIdentifier</key>
    <string>$BUNDLE_ID</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>$APP_NAME</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>$VERSION</string>
    <key>CFBundleVersion</key>
    <string>$VERSION</string>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.music-games</string>
    <key>LSMinimumSystemVersion</key>
    <string>$MINIMUM_MACOS</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
PLIST
printf 'APPL????' > "$APP/Contents/PkgInfo"
plutil -lint "$APP/Contents/Info.plist" > /dev/null
echo "$APP"
