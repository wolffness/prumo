#!/bin/zsh
# Package tuxedo as a macOS .app bundle (dist/Tuxedo.app).
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

cat > "$APP/Contents/MacOS/tuxedo-launcher" <<'LAUNCHER'
#!/bin/zsh
set -euo pipefail
BIN="$(cd "$(dirname "$0")/../Resources" && pwd)/tuxedo"
# A "Tuxedo" Terminal profile (font + colors) is applied when present;
# without one the window keeps the user's default profile. The profile is
# created on demand so a fresh machine still launches fine.
/usr/bin/osascript <<EOF
tell application "Terminal"
    activate
    set t to do script "cd \"\$HOME\"; clear; exec '$BIN'"
    if exists settings set "Tuxedo" then
        set current settings of t to settings set "Tuxedo"
    end if
end tell
EOF
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
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
PLIST

echo "Bundle created: $APP"
echo "Install with:   cp -R $APP /Applications/"
