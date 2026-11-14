#!/bin/sh
# Installs the built program into the application menu of a Linux desktop:
# the executable, the desktop entry and the icon.
#
#   cargo build --release
#   tools/install-linux.sh                  for the current user, into ~/.local
#   sudo PREFIX=/usr/local tools/install-linux.sh        for all users
#   tools/install-linux.sh --uninstall      removes what was installed (same PREFIX)
#
# Under Wayland the desktop entry is also what gives the window its icon in the dock
# and in the window switcher.
set -eu

prefix=${PREFIX:-"$HOME/.local"}
root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)

binary="$prefix/bin/rucheckers"
entry="$prefix/share/applications/rucheckers.desktop"
icon="$prefix/share/icons/hicolor/256x256/apps/rucheckers.png"

# Menus and icon themes keep caches; the tools that rebuild them are optional.
refresh() {
    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database -q "$prefix/share/applications" || true
    fi
    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        gtk-update-icon-cache -q -t -f "$prefix/share/icons/hicolor" || true
    fi
}

case "${1:-}" in
"")
    built="$root/target/release/rucheckers"
    if [ ! -x "$built" ]; then
        echo "$built is missing: run 'cargo build --release' first" >&2
        exit 1
    fi
    install -Dm755 "$built" "$binary"
    install -Dm644 "$root/assets/icons/rucheckers.png" "$icon"
    # The menu starts the program by its full path: the directory of the executable
    # is not on the search path of every desktop session.
    mkdir -p "$(dirname -- "$entry")"
    sed "s|^Exec=rucheckers|Exec=\"$binary\"|" "$root/assets/linux/rucheckers.desktop" >"$entry"
    chmod 644 "$entry"
    refresh
    echo "Installed: $binary"
    ;;
--uninstall)
    rm -f -- "$binary" "$entry" "$icon"
    refresh
    echo "Removed: $binary"
    ;;
*)
    echo "usage: $0 [--uninstall]" >&2
    exit 2
    ;;
esac
