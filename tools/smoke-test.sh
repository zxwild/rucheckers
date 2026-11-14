#!/bin/sh
# Starts the built program on a virtual X display and checks that its window opens
# and stays open. CI runs this on every Linux distribution it builds on.
#
#   tools/smoke-test.sh [path/to/rucheckers] [screenshot.png]
#
# Needs Xvfb, xdotool and the Mesa software renderer; a screenshot also needs
# ImageMagick.
set -eu

binary=${1:-target/release/rucheckers}
screenshot=${2:-}
display=:${SMOKE_DISPLAY:-93}

work=$(mktemp -d)
xvfb=
app=
cleanup() {
    [ -z "$app" ] || kill "$app" 2>/dev/null || true
    [ -z "$xvfb" ] || kill "$xvfb" 2>/dev/null || true
    rm -rf "$work"
}
trap cleanup EXIT

fail() {
    echo "smoke test failed: $1" >&2
    if [ -s "$work/app.log" ]; then
        sed 's/^/    /' "$work/app.log" >&2
    fi
    exit 1
}

Xvfb "$display" -screen 0 1280x900x24 -nolisten tcp >"$work/xvfb.log" 2>&1 &
xvfb=$!

# The program draws with the software renderer and keeps its settings out of the home
# directory. The language is fixed so that the screenshot does not depend on the machine.
unset WAYLAND_DISPLAY
export DISPLAY="$display" LIBGL_ALWAYS_SOFTWARE=1 XDG_CONFIG_HOME="$work/config" LC_ALL=en_US.UTF-8

ready=
for _ in 1 2 3 4 5 6 7 8 9 10; do
    if xdotool getdisplaygeometry >/dev/null 2>&1; then
        ready=1
        break
    fi
    sleep 1
done
[ -n "$ready" ] || fail "the virtual display did not start"

"$binary" >"$work/app.log" 2>&1 &
app=$!

window=
for _ in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20; do
    sleep 1
    kill -0 "$app" 2>/dev/null || fail "the program exited"
    if xdotool search --class rucheckers >/dev/null 2>&1; then
        window=1
        break
    fi
done
[ -n "$window" ] || fail "the window did not appear"

# A program that cannot draw exits within the first frames.
sleep 3
kill -0 "$app" 2>/dev/null || fail "the program exited after opening its window"

if [ -n "$screenshot" ]; then
    if command -v import >/dev/null 2>&1; then
        import -window root "$screenshot"
    else
        magick import -window root "$screenshot"
    fi
fi
echo "smoke test passed: the window is open"
