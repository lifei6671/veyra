#!/bin/sh
# Isolated P0 app bundle; no install, network business requests, or second UI loop.
set -eu
cd "$(dirname "$0")/../.."
export MACOSX_DEPLOYMENT_TARGET=15.0
cargo +1.99.0 build --locked -p veyra-gpui-prototype
bundle="$(pwd)/target/p0-03/VeyraPrototype.app"
mkdir -p "$bundle/Contents/MacOS"
cp target/debug/veyra-gpui-prototype "$bundle/Contents/MacOS/"
python3 - "$bundle" <<'PY'
import pathlib, plistlib, sys
path = pathlib.Path(sys.argv[1]) / 'Contents' / 'Info.plist'
path.write_bytes(plistlib.dumps({
    'CFBundleExecutable': 'veyra-gpui-prototype',
    'CFBundleIdentifier': 'me.veyra.gpui-prototype',
    'CFBundleName': 'VeyraPrototype',
    'CFBundlePackageType': 'APPL',
    'LSMinimumSystemVersion': '15.0',
    'NSHighResolutionCapable': True,
}))
PY
codesign --force --sign - "$bundle"
printf 'App bundle: %s\n' "$bundle"
