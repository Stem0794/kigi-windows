#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ "$(uname -s)" != Darwin ]]; then
  echo "Le packaging et la signature nécessitent macOS et les outils Xcode." >&2
  exit 1
fi
mode="${1:-native}"
case "$mode" in
  native) targets=("$(rustc -vV | sed -n 's/^host: //p')") ;;
  universal) targets=(x86_64-apple-darwin aarch64-apple-darwin) ;;
  *) echo "Usage : bash scripts/bundle.sh [native|universal]" >&2; exit 1 ;;
esac
for target in "${targets[@]}"; do
  MACOSX_DEPLOYMENT_TARGET=11.0 cargo build --locked --release --target "$target"
done
bundle="$PWD/dist/Kigi Windows.app"
mkdir -p "$bundle/Contents/MacOS"
if [[ "$mode" == universal ]]; then
  lipo -create target/x86_64-apple-darwin/release/kigi-windows \
    target/aarch64-apple-darwin/release/kigi-windows \
    -output "$bundle/Contents/MacOS/kigi-windows"
else
  cp "target/${targets[0]}/release/kigi-windows" "$bundle/Contents/MacOS/kigi-windows"
fi
cp resources/Info.plist "$bundle/Contents/Info.plist"
plutil -lint "$bundle/Contents/Info.plist"
# Local development only. Not a Developer ID signature or notarization.
codesign --force --sign - "$bundle"
codesign --verify --strict "$bundle"
echo "Prototype local : $bundle"
echo 'Lancement : open "dist/Kigi Windows.app"'
