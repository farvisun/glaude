#!/usr/bin/env bash
# Build Glaude and install it to /Applications (macOS).
#
#   scripts/install-macos.sh             build, then install
#   scripts/install-macos.sh --no-build  install the last build
#
# The bundle is moved, not copied: a second Glaude.app left in target/
# would show up in Spotlight and Launchpad next to the installed one.
set -euo pipefail
cd "$(dirname "$0")/.."

app="$PWD/target/release/bundle/macos/Glaude.app"
dest=/Applications/Glaude.app
lsregister=/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister

if pgrep -f "^$dest/Contents/MacOS/glaude" >/dev/null; then
  echo "Glaude is running. Quit it first, then run this again." >&2
  exit 1
fi

if [ "${1:-}" != "--no-build" ]; then
  bunx @tauri-apps/cli@2 build --bundles app
fi
if [ ! -d "$app" ]; then
  echo "No bundle at $app; build first." >&2
  exit 1
fi

rm -rf "$dest.new"
mv "$app" "$dest.new"
# Ad-hoc signature (no Apple developer certificate); re-sign after the move.
codesign --force --deep --sign - "$dest.new"
rm -rf "$dest"
mv "$dest.new" "$dest"
xattr -dr com.apple.quarantine "$dest" 2>/dev/null || true
codesign --verify --deep --strict "$dest"

# Forget the build copy, register the installed one.
"$lsregister" -u "$app" >/dev/null 2>&1 || true
"$lsregister" -f "$dest"

echo "Installed $dest $(defaults read "$dest/Contents/Info.plist" CFBundleShortVersionString)"
