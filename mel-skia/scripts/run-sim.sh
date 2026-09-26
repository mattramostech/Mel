#!/usr/bin/env bash
# One-command Simulator flow (ADR 002): build a numbered example for the iOS
# Simulator, wrap it into a .app bundle, install it and launch it.
#
#   scripts/run-sim.sh 002_blank_app
#   BUILD_MODE=release scripts/run-sim.sh 006_frame_timing
#
# Blocks while the app runs, streaming its stdout/stderr here (Ctrl-C
# detaches the console; the app keeps running in the Simulator). Requires
# Xcode; the Rust target comes from rust-toolchain.toml. Apple Silicon only
# (the target is hard-coded aarch64) until someone needs otherwise.
set -euo pipefail

if [[ $# -ne 1 ]]; then
    echo "usage: $0 <example>    e.g. $0 002_blank_app" >&2
    exit 1
fi

EXAMPLE="$1"
if [[ ! "$EXAMPLE" =~ ^[0-9]{3}_[a-z0-9_]+$ ]]; then
    echo "error: expected a numbered example such as 006_frame_timing" >&2
    exit 1
fi
BUILD_MODE="${BUILD_MODE:-debug}"
BUILD_ARGS=()
case "$BUILD_MODE" in
    debug) ;;
    release) BUILD_ARGS+=(--release) ;;
    *) echo "error: BUILD_MODE must be debug or release" >&2; exit 1 ;;
esac
TARGET="aarch64-apple-ios-sim"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# Underscores are invalid in bundle identifiers.
BUNDLE_ID="dev.mel.example.${EXAMPLE//_/-}"

cd "$ROOT"

# 1. Build the example.
# The guarded array expansion also supports macOS Bash 3.2 with nounset.
cargo build --locked --target "$TARGET" --target-dir target -p mel-platform \
    --example "$EXAMPLE" ${BUILD_ARGS[@]+"${BUILD_ARGS[@]}"}

# 2. Assemble the .app bundle. An iOS app bundle is just a directory holding
#    the binary and an Info.plist; the Simulator requires no code signature.
APP_DIR="target/sim/$EXAMPLE.app"
rm -rf "$APP_DIR"
mkdir -p "$APP_DIR"
cp "target/$TARGET/$BUILD_MODE/examples/$EXAMPLE" "$APP_DIR/$EXAMPLE"

# UILaunchScreen = {} opts into a runtime-generated launch screen — without
# it, UIKit letterboxes apps that lack a launch storyboard instead of going
# edge-to-edge full screen.
cat > "$APP_DIR/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleIdentifier</key>        <string>${BUNDLE_ID}</string>
    <key>CFBundleExecutable</key>        <string>${EXAMPLE}</string>
    <key>CFBundleName</key>              <string>${EXAMPLE}</string>
    <key>CFBundlePackageType</key>       <string>APPL</string>
    <key>CFBundleVersion</key>           <string>1</string>
    <key>CFBundleShortVersionString</key> <string>0.1.0</string>
    <key>CFBundleSupportedPlatforms</key> <array><string>iPhoneSimulator</string></array>
    <key>MinimumOSVersion</key>          <string>15.0</string>
    <key>UILaunchScreen</key>            <dict/>
    <!-- Rotation is part of the platform contract (feature 004): declare the
         supported orientations explicitly instead of relying on UIKit's
         all-but-upside-down default. -->
    <key>UISupportedInterfaceOrientations</key>
    <array>
        <string>UIInterfaceOrientationPortrait</string>
        <string>UIInterfaceOrientationLandscapeLeft</string>
        <string>UIInterfaceOrientationLandscapeRight</string>
    </array>
</dict>
</plist>
PLIST

# 3. Ensure a booted Simulator (reuse one if already running).
if ! xcrun simctl list devices booted | grep -q '(Booted)'; then
    UDID="$(xcrun simctl list devices available \
        | sed -n 's/.*iPhone[^(]*(\([0-9A-F-]\{36\}\)).*/\1/p' | head -1)"
    if [[ -z "$UDID" ]]; then
        echo "error: no available iPhone Simulator; create one in Xcode" >&2
        exit 1
    fi
    echo "Booting Simulator $UDID ..."
    xcrun simctl boot "$UDID"
fi
open -a Simulator

# 4. Install and launch, attaching the app's console to this terminal.
xcrun simctl install booted "$APP_DIR"
echo "Launching $BUNDLE_ID ..."
exec xcrun simctl launch --console-pty --terminate-running-process booted "$BUNDLE_ID"
