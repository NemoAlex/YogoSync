#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
APP='target/release/bundle/macos/YOGO Pet.app'
VERSION=$(node -p "JSON.parse(require('fs').readFileSync('src-tauri/tauri.conf.json')).version")
mkdir -p dist
STAGE=$(mktemp -d "${TMPDIR:-/tmp}/yogo-package.XXXXXX")
trap 'rm -rf "$STAGE"' EXIT HUP INT TERM
DMG="dist/YOGO-Pet-$VERSION-macOS-arm64.dmg"
test -d "$APP"
mkdir -p "$STAGE/Codex 插件/yogo-pet/.codex-plugin" "$STAGE/Codex 插件/yogo-pet/hooks" "$STAGE/Codex 插件/yogo-pet/bin"
ditto "$APP" "$STAGE/YOGO Pet.app"
# Local builds need a complete ad-hoc signature after resources are bundled.
if ! codesign --verify --deep --strict "$STAGE/YOGO Pet.app" 2>/dev/null; then
  codesign --force --deep --sign - "$STAGE/YOGO Pet.app"
fi
codesign --verify --deep --strict "$STAGE/YOGO Pet.app"
cp src-tauri/resources/plugin.json "$STAGE/Codex 插件/yogo-pet/.codex-plugin/plugin.json"
cp src-tauri/resources/hooks.json "$STAGE/Codex 插件/yogo-pet/hooks/hooks.json"
cp src-tauri/resources/yogo-pet-hook "$STAGE/Codex 插件/yogo-pet/bin/yogo-pet-hook"
cp README.md "$STAGE/使用说明.md"
ln -sfn /Applications "$STAGE/Applications"
hdiutil create -volname 'YOGO Pet' -srcfolder "$STAGE" -format UDZO -ov "$DMG"
shasum -a 256 "$DMG" > "$DMG.sha256"
