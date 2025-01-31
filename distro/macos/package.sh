#!/bin/sh
set -eu

APP_NAME="Lattice"
BIN="lattice"
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
cd "$ROOT"

TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
APP="$TARGET_DIR/$APP_NAME.app"
CONTENTS="$APP/Contents"
IDENTITY="${CODESIGN_IDENTITY:--}"
VERSION="$(cargo pkgid | sed 's/.*[#@]//')"
ZIP="$TARGET_DIR/$APP_NAME-$VERSION.zip"

echo "==> cargo build --release"
cargo build --release --locked

echo "==> assembling $APP_NAME.app ($VERSION)"
rm -rf "$APP"
mkdir -p "$CONTENTS/MacOS"
cp "$HERE/Info.plist" "$CONTENTS/Info.plist"
/usr/libexec/PlistBuddy \
    -c "Set :CFBundleVersion $VERSION" \
    -c "Set :CFBundleShortVersionString $VERSION" \
    "$CONTENTS/Info.plist"
cp "$TARGET_DIR/release/$BIN" "$CONTENTS/MacOS/$BIN"

echo "==> codesign (identity: $IDENTITY)"
codesign --force --sign "$IDENTITY" "$APP"
codesign --verify "$APP"

echo "==> zipping $ZIP"
rm -f "$ZIP"
ditto -c -k --keepParent "$APP" "$ZIP"

echo
echo "Built $APP"
echo "  Install:  cp -R \"$APP\" /Applications/"
echo "  Run:      open \"$APP\""
