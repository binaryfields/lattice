#!/bin/sh
set -eu

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
cd "$ROOT"

BIN="$(sed -n 's/^name *= *"\(.*\)"/\1/p' Cargo.toml | head -1)"
VERSION="$(sed -n 's/^version *= *"\(.*\)"/\1/p' Cargo.toml | head -1)"
APP_NAME="$(echo "$BIN" | awk '{print toupper(substr($0,1,1)) substr($0,2)}')"

TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
APP="$TARGET_DIR/$APP_NAME.app"
CONTENTS="$APP/Contents"
ZIP="$TARGET_DIR/$APP_NAME-$VERSION.zip"

cargo build --release --locked

rm -rf "$APP"
mkdir -p "$CONTENTS/MacOS"
cp "$HERE/Info.plist" "$CONTENTS/Info.plist"
/usr/libexec/PlistBuddy \
    -c "Add :CFBundleVersion string $VERSION" \
    -c "Add :CFBundleShortVersionString string $VERSION" \
    "$CONTENTS/Info.plist"
cp "$TARGET_DIR/release/$BIN" "$CONTENTS/MacOS/$BIN"

codesign --force --sign - "$APP"

rm -f "$ZIP"
ditto -c -k --keepParent "$APP" "$ZIP"

echo "$ZIP"
