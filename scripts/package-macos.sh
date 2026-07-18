#!/bin/zsh
# Package prumo as a macOS .app bundle and install it to /Applications.
#
# The bundle wraps the release binary in a native AppKit launcher that
# opens it in an iTerm2/Terminal window with the phosphor profile. The
# terminal session runs through the user's login shell, so TODO_FILE /
# TODO_DIR from dotfiles are honored; with neither set, prumo starts in
# $HOME (opening ~/todo.txt or the first-run welcome).
set -euo pipefail

cd "$(dirname "$0")/.."

echo "Building release binary..."
cargo build --release

APP=dist/Prumo.app
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

# The Rust crate keeps the upstream name (tuxedo) so merges stay cheap;
# only the copy inside the bundle takes the Prumo name.
cp target/release/tuxedo "$APP/Contents/Resources/prumo"
cp packaging/prumo.icns "$APP/Contents/Resources/prumo.icns"
# Native quick-capture agent (⌥]): a tiny AppKit panel that appends to
# the inbox.txt sibling of TODO_FILE, kept alive by a per-user
# LaunchAgent. It lives in its OWN nested .app with its own bundle
# identifier — if it ran as a bare binary inside Prumo.app,
# LaunchServices would count it as "Prumo is already running" and
# Dock/Finder launches of the main app would fail with error -600.
echo "Building Prumo agent (capture + menu bar)..."
AGENTAPP="$APP/Contents/Resources/PrumoAgent.app"
mkdir -p "$AGENTAPP/Contents/MacOS"
swiftc -O -o "$AGENTAPP/Contents/MacOS/PrumoAgent" \
    packaging/agent/Paths.swift \
    packaging/agent/Theme.swift \
    packaging/agent/Summary.swift \
    packaging/agent/TaskRowView.swift \
    packaging/agent/CapturePanel.swift \
    packaging/agent/MenuBar.swift \
    packaging/agent/main.swift \
    -framework AppKit -framework Carbon
cat > "$AGENTAPP/Contents/Info.plist" <<CAPPLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>Prumo Agent</string>
    <key>CFBundleIdentifier</key>
    <string>dev.wolffness.prumo.agent</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleExecutable</key>
    <string>PrumoAgent</string>
    <key>LSUIElement</key>
    <true/>
</dict>
</plist>
CAPPLIST

# The iTerm2-based capture profile is superseded by the native panel;
# remove it so ⌥] isn't registered twice.
rm -f "$HOME/Library/Application Support/iTerm2/DynamicProfiles/tuxedo-capture.json"

# Native launcher: a real AppKit executable, so the Dock icon stops
# bouncing (shell-script launchers never report "finished launching"),
# shows a running dot while the TUI is open, and refreshes the iTerm2
# profile with this bundle's binary path on every launch.
echo "Building launcher..."
swiftc -O -o "$APP/Contents/MacOS/prumo-launcher" \
    packaging/PrumoLauncher.swift -framework AppKit

VERSION=$(grep -m1 '^version' Cargo.toml | sed 's/.*"\(.*\)"/\1/')
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>Prumo</string>
    <key>CFBundleDisplayName</key>
    <string>Prumo</string>
    <key>CFBundleIdentifier</key>
    <string>dev.wolffness.prumo</string>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleExecutable</key>
    <string>prumo-launcher</string>
    <key>CFBundleIconFile</key>
    <string>prumo</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSAppleEventsUsageDescription</key>
    <string>Prumo opens its task list in an iTerm2 or Terminal window.</string>
</dict>
</plist>
PLIST

codesign --force --deep -s - "$APP"

# Install to /Applications and drop the staging copy so only one Prumo.app
# exists on the machine (a stray dist copy kept showing up in Finder search).
rm -rf /Applications/Prumo.app
cp -R "$APP" /Applications/
rm -rf "$APP"
touch /Applications/Prumo.app

# Kill any launcher still running from a PREVIOUS install. Reinstalling only
# replaces the binary on disk; a launcher already in memory keeps handling
# Dock/"open" launches with its old code (LaunchServices routes to the running
# instance), so a fix never takes effect until the stale process is gone.
# The launcher doesn't own the TUI window (iTerm/Terminal does), so killing it
# never closes an open task list; the next launch just spawns the new binary.
pkill -f "Contents/MacOS/prumo-launcher" 2>/dev/null || true

# Migrate legacy installs from the Tuxedo era: old app bundle, old capture
# agent, and the old unified agent under the tuxedo bundle id.
rm -rf /Applications/Tuxedo.app
pkill -f "Contents/MacOS/tuxedo-launcher" 2>/dev/null || true
OLD_AGENT="$HOME/Library/LaunchAgents/dev.wolffness.tuxedo.capture.plist"
launchctl bootout "gui/$(id -u)/dev.wolffness.tuxedo.capture" 2>/dev/null || true
pkill -f "Resources/TuxedoCapture" 2>/dev/null || true
rm -f "$OLD_AGENT"
launchctl bootout "gui/$(id -u)/dev.wolffness.tuxedo.agent" 2>/dev/null || true
pkill -f "Resources/TuxedoAgent" 2>/dev/null || true
rm -f "$HOME/Library/LaunchAgents/dev.wolffness.tuxedo.agent.plist"
rm -f "$HOME/Library/Application Support/iTerm2/DynamicProfiles/tuxedo.json" \
      "$HOME/Library/Application Support/iTerm2/DynamicProfiles/tuxedo-capture.json"

AGENT="$HOME/Library/LaunchAgents/dev.wolffness.prumo.agent.plist"
mkdir -p "$HOME/Library/LaunchAgents"
cat > "$AGENT" <<AGENTPLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>dev.wolffness.prumo.agent</string>
    <key>ProgramArguments</key>
    <array>
        <string>/Applications/Prumo.app/Contents/Resources/PrumoAgent.app/Contents/MacOS/PrumoAgent</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
</dict>
</plist>
AGENTPLIST
launchctl bootout "gui/$(id -u)/dev.wolffness.prumo.agent" 2>/dev/null || true
pkill -f "Resources/PrumoAgent" 2>/dev/null || true
sleep 1
launchctl bootstrap "gui/$(id -u)" "$AGENT" || \
    launchctl kickstart -k "gui/$(id -u)/dev.wolffness.prumo.agent" || true
echo "Installed: /Applications/Prumo.app (+ agent: ⌥] capture & menu bar)"
