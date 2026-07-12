#!/bin/zsh
# Package tuxedo as a macOS .app bundle and install it to /Applications.
#
# The bundle wraps the release binary in a launcher that opens it in a
# dedicated Terminal window, so tuxedo gets a Dock icon, Spotlight entry,
# and Finder presence without any code changes. `do script` runs in the
# user's login shell, so TODO_FILE / TODO_DIR from dotfiles are honored;
# with neither set, the launcher starts in $HOME (tuxedo then opens
# ~/todo.txt or shows the first-run welcome).
set -euo pipefail

cd "$(dirname "$0")/.."

echo "Building release binary..."
cargo build --release

APP=dist/Tuxedo.app
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

cp target/release/tuxedo "$APP/Contents/Resources/tuxedo"
cp packaging/tuxedo.icns "$APP/Contents/Resources/tuxedo.icns"
cp scripts/capture-inbox.sh "$APP/Contents/Resources/tuxedo-capture.sh"
chmod +x "$APP/Contents/Resources/tuxedo-capture.sh"

# Quick-capture hotkey window (⌥Space): its own dynamic profile so iTerm2
# registers the global hotkey. Key code 49 = Space; modifier 524288 = ⌥.
DYN_DIR="$HOME/Library/Application Support/iTerm2/DynamicProfiles"
mkdir -p "$DYN_DIR"
cat > "$DYN_DIR/tuxedo-capture.json" <<PROFILE
{
  "Profiles": [
    {
      "Name": "Tuxedo Capture",
      "Guid": "tuxedo-capture-hotkey",
      "Normal Font": "IBMPlexMono-Regular 15",
      "Use Non-ASCII Font": false,
      "Custom Command": "Yes",
      "Command": "/bin/zsh -lc 'exec \\"/Applications/Tuxedo.app/Contents/Resources/tuxedo-capture.sh\\"'",
      "Has Hotkey": true,
      "HotKey Key Code": 49,
      "HotKey Modifier Flags": 524288,
      "HotKey Characters": " ",
      "HotKey Characters Ignoring Modifiers": " ",
      "HotKey Window Autohides": true,
      "HotKey Window Animates": true,
      "HotKey Window Floats": true,
      "Rows": 6,
      "Columns": 72,
      "Background Color": { "Red Component": 0.008, "Green Component": 0.04, "Blue Component": 0.008 },
      "Foreground Color": { "Red Component": 0.2, "Green Component": 1.0, "Blue Component": 0.2 },
      "Cursor Color": { "Red Component": 0.2, "Green Component": 1.0, "Blue Component": 0.2 },
      "Cursor Text Color": { "Red Component": 0.008, "Green Component": 0.04, "Blue Component": 0.008 },
      "Silence Bell": true,
      "Scrollback Lines": 0
    }
  ]
}
PROFILE

cat > "$APP/Contents/MacOS/tuxedo-launcher" <<'LAUNCHER'
#!/bin/zsh
set -euo pipefail
BIN="$(cd "$(dirname "$0")/../Resources" && pwd)/tuxedo"

if [[ -d /Applications/iTerm.app ]]; then
    # Install/refresh the "Tuxedo" dynamic profile (font + phosphor colors)
    # — iTerm2 picks up DynamicProfiles JSON automatically. The profile's
    # Command uses a login shell so TODO_FILE/TODO_DIR are honored, and
    # resolves the app-bundled binary path at launch time.
    DYN_DIR="$HOME/Library/Application Support/iTerm2/DynamicProfiles"
    mkdir -p "$DYN_DIR"
    cat > "$DYN_DIR/tuxedo.json" <<PROFILE
{
  "Profiles": [
    {
      "Name": "Tuxedo",
      "Guid": "tuxedo-phosphor-green",
      "Normal Font": "IBMPlexMono-Regular 15",
      "Use Non-ASCII Font": false,
      "Custom Command": "Yes",
      "Command": "/bin/zsh -lc 'cd \\"\$HOME\\"; exec \\"$BIN\\"'",
      "Background Color": { "Red Component": 0.008, "Green Component": 0.04, "Blue Component": 0.008 },
      "Foreground Color": { "Red Component": 0.2, "Green Component": 1.0, "Blue Component": 0.2 },
      "Bold Color": { "Red Component": 0.4, "Green Component": 1.0, "Blue Component": 0.4 },
      "Cursor Color": { "Red Component": 0.2, "Green Component": 1.0, "Blue Component": 0.2 },
      "Cursor Text Color": { "Red Component": 0.008, "Green Component": 0.04, "Blue Component": 0.008 },
      "Silence Bell": true
    }
  ]
}
PROFILE
    /usr/bin/osascript <<EOF
tell application "iTerm2"
    activate
    -- iTerm2 loads DynamicProfiles asynchronously after startup, so a
    -- cold launch may not know "Tuxedo" yet: retry briefly before falling
    -- back to a default window (which would lose the font/colors).
    set opened to false
    repeat with i from 1 to 20
        try
            create window with profile "Tuxedo"
            set opened to true
            exit repeat
        on error
            delay 0.25
        end try
    end repeat
    if not opened then
        set w to (create window with default profile)
        tell current session of w to write text "cd \"\$HOME\"; clear; exec '$BIN'"
    end if
end tell
EOF
    # Stay alive while tuxedo runs so the Dock shows the app as open
    # (otherwise the launcher exits in milliseconds and the icon vanishes
    # before it can be pinned).
    sleep 3
    while pgrep -f "$BIN" >/dev/null 2>&1; do
        sleep 5
    done
    exit 0
fi

# Fallback: Terminal.app, applying the "Tuxedo" settings set when present.
/usr/bin/osascript <<EOF
tell application "Terminal"
    activate
    set t to do script "cd \"\$HOME\"; clear; exec '$BIN'"
    if exists settings set "Tuxedo" then
        set current settings of t to settings set "Tuxedo"
    end if
end tell
EOF
sleep 3
while pgrep -f "$BIN" >/dev/null 2>&1; do
    sleep 5
done
LAUNCHER
chmod +x "$APP/Contents/MacOS/tuxedo-launcher"

VERSION=$(grep -m1 '^version' Cargo.toml | sed 's/.*"\(.*\)"/\1/')
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>Tuxedo</string>
    <key>CFBundleDisplayName</key>
    <string>Tuxedo</string>
    <key>CFBundleIdentifier</key>
    <string>dev.wolffness.tuxedo</string>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleExecutable</key>
    <string>tuxedo-launcher</string>
    <key>CFBundleIconFile</key>
    <string>tuxedo</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
PLIST

# Install to /Applications and drop the staging copy so only one Tuxedo.app
# exists on the machine (a stray dist copy kept showing up in Finder search).
rm -rf /Applications/Tuxedo.app
cp -R "$APP" /Applications/
rm -rf "$APP"
touch /Applications/Tuxedo.app
echo "Installed: /Applications/Tuxedo.app"
