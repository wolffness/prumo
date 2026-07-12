#!/bin/zsh
# Quick-capture prompt for the Tuxedo hotkey window (iTerm2, ⌥Space).
#
# Each line typed is appended to the inbox.txt sibling of the todo file;
# a running tuxedo drains the inbox automatically (with natural-language
# parsing), and a closed one drains it on next launch. The window hides
# itself after each entry — press the hotkey again for the next thought.
set -u

TODO="${TODO_FILE:-$HOME/todo.txt}"
INBOX="$(dirname "$TODO")/inbox.txt"

# Phosphor-green retro prompt.
GRN=$'\e[38;2;51;255;51m'
DIM=$'\e[38;2;29;143;29m'
RST=$'\e[0m'

while true; do
    clear
    printf "%s TUXEDO CAPTURE %s→ %s\n\n" "$DIM" "$INBOX" "$RST"
    printf "%s> %s" "$GRN" "$RST"
    IFS= read -r task || exit 0
    if [[ -n "${task// /}" ]]; then
        print -r -- "$task" >> "$INBOX"
        printf "\n%s  ✓ adicionada%s\n" "$DIM" "$RST"
    fi
    sleep 0.35
    # Hide the hotkey window until the next ⌥Space. Best-effort: if the
    # AppleScript API is unavailable the window simply stays open.
    /usr/bin/osascript -e 'tell application "iTerm2" to hide hotkey window' >/dev/null 2>&1
done
