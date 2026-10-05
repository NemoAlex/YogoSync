#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
APP='target/release/bundle/macos/YogoSync.app'
VERSION=$(node -p "JSON.parse(require('fs').readFileSync('src-tauri/tauri.conf.json')).version")
mkdir -p dist
STAGE=$(mktemp -d "${TMPDIR:-/tmp}/yogo-package.XXXXXX")
trap 'rm -rf "$STAGE"' EXIT HUP INT TERM
DMG="dist/YogoSync-$VERSION-macOS-arm64.dmg"
test -d "$APP"
ditto "$APP" "$STAGE/YogoSync.app"
# Local builds need a complete ad-hoc signature after resources are bundled.
if ! codesign --verify --deep --strict "$STAGE/YogoSync.app" 2>/dev/null; then
  codesign --force --deep --sign - "$STAGE/YogoSync.app"
fi
codesign --verify --deep --strict "$STAGE/YogoSync.app"
cp README.md "$STAGE/README.md"
cp README.zh-CN.md "$STAGE/README.zh-CN.md"
ln -sfn /Applications "$STAGE/Applications"
hdiutil create -volname 'YogoSync' -srcfolder "$STAGE" -format UDZO -ov "$DMG"
(cd dist && shasum -a 256 "$(basename "$DMG")" > "$(basename "$DMG").sha256")
