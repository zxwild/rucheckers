#!/bin/sh
# Starts the built program on a virtual display and checks that it opens its window,
# draws into it and stays open. CI runs this on every Linux distribution it builds on,
# once for each of the two display systems.
#
#   tools/smoke-test.sh x11     [path/to/rucheckers] [screenshot.png]
#   tools/smoke-test.sh wayland [path/to/rucheckers] [screenshot.png]
#
# Both modes need the Mesa software renderer and ImageMagick. `x11` also needs Xvfb and
# xdotool; `wayland` needs the Weston compositor.
#
# The test looks at the screen: an empty window is one flat colour, the board with its
# pieces has thousands. Under Wayland it first watches the program hand finished frames
# to the compositor. A Weston older than version 12 is not always able to capture the
# screen without a monitor; there the frames decide alone.
set -eu

mode=${1:-x11}
binary=${2:-target/release/rucheckers}
screenshot=${3:-}
case "$mode" in
x11 | wayland) ;;
*)
    echo "usage: $0 x11|wayland [path/to/rucheckers] [screenshot.png]" >&2
    exit 2
    ;;
esac

work=$(mktemp -d)
server=
app=
cleanup() {
    [ -z "$app" ] || kill "$app" 2>/dev/null || true
    [ -z "$server" ] || kill "$server" 2>/dev/null || true
    rm -rf "$work"
}
trap cleanup EXIT

fail() {
    echo "smoke test ($mode) failed: $1" >&2
    if [ -s "$work/app.log" ]; then
        # Lines in square brackets are the Wayland protocol trace.
        echo "--- program" >&2
        grep -v '^\[' "$work/app.log" | tail -n 40 | sed 's/^/    /' >&2
    fi
    if [ "$mode" = wayland ] && [ -s "$work/app.log" ]; then
        echo "--- last protocol messages" >&2
        grep '^\[' "$work/app.log" | tail -n 25 | sed 's/^/    /' >&2
    fi
    if [ -s "$work/server.log" ]; then
        echo "--- display server" >&2
        tail -n 40 "$work/server.log" | sed 's/^/    /' >&2
    fi
    exit 1
}

# ImageMagick 7 keeps the classic command names only on some distributions.
magick_tool() {
    tool=$1
    shift
    if command -v "$tool" >/dev/null 2>&1; then
        "$tool" "$@"
    else
        magick "$tool" "$@"
    fi
}

# The program draws with the software renderer and keeps its settings out of the home
# directory. The language is fixed so that the picture does not depend on the machine.
export LIBGL_ALWAYS_SOFTWARE=1 XDG_CONFIG_HOME="$work/config" LC_ALL=en_US.UTF-8
shot="$work/screen.png"

if [ "$mode" = x11 ]; then
    display=:${SMOKE_DISPLAY:-93}
    Xvfb "$display" -screen 0 1280x900x24 -nolisten tcp >"$work/server.log" 2>&1 &
    server=$!
    unset WAYLAND_DISPLAY
    export DISPLAY="$display"
    ready=
    for _ in 1 2 3 4 5 6 7 8 9 10; do
        if xdotool getdisplaygeometry >/dev/null 2>&1; then
            ready=1
            break
        fi
        sleep 1
    done
    [ -n "$ready" ] || fail "the virtual display did not start"
else
    export XDG_RUNTIME_DIR="$work/runtime"
    mkdir -m 700 "$XDG_RUNTIME_DIR"
    socket=rucheckers-smoke
    # Starts Weston with the given options and waits for its socket.
    start_weston() {
        weston "$@" --socket="$socket" --width=1280 --height=900 --debug >"$work/server.log" 2>&1 &
        server=$!
        for _ in 1 2 3 4 5 6 7 8 9 10; do
            [ ! -S "$XDG_RUNTIME_DIR/$socket" ] || return 0
            kill -0 "$server" 2>/dev/null || return 1
            sleep 1
        done
        return 1
    }
    # The pixman renderer composes the screen in memory, so that it can be captured; the
    # kiosk shell shows the only window full-screen, without decorations. Weston before
    # version 12 spells the options differently, and the window gets its frames
    # reliably only under the ordinary desktop shell there.
    start_weston --backend=headless --renderer=pixman --shell=kiosk ||
        start_weston --backend=headless-backend.so --use-pixman ||
        fail "the compositor did not start"
    unset DISPLAY
    export WAYLAND_DISPLAY="$socket"
    weston_version=$(weston --version | sed -n 's/^weston \([0-9][0-9]*\).*/\1/p')
    # The program prints every protocol message it exchanges with the compositor.
    export WAYLAND_DEBUG=1
fi

"$binary" >"$work/app.log" 2>&1 &
app=$!

# Takes a picture of the screen; fails quietly while there is nothing to capture.
capture() {
    rm -f "$shot"
    if [ "$mode" = x11 ]; then
        magick_tool import -window root "$shot" 2>/dev/null
    else
        (cd "$work" && rm -f wayland-screenshot-*.png && weston-screenshooter >/dev/null 2>&1) || return 1
        mv "$work"/wayland-screenshot-*.png "$shot" 2>/dev/null
    fi
}

# Frames the program has handed to the Wayland compositor: buffers attached to its surface.
frames() {
    grep -c 'attach(wl_buffer' "$work/app.log" || true
}

colours=0
drawn=
for _ in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20; do
    sleep 1
    kill -0 "$app" 2>/dev/null || fail "the program exited"
    if [ "$mode" = x11 ]; then
        xdotool search --class rucheckers >/dev/null 2>&1 || continue
    else
        [ "$(frames)" -ge 3 ] || continue
    fi
    if capture; then
        colours=$(magick_tool identify -format '%k' "$shot")
    fi
    if [ "$colours" -ge 1000 ]; then
        drawn="$colours colours on the screen"
        break
    fi
    if [ "$mode" = wayland ] && [ "${weston_version:-0}" -lt 12 ]; then
        drawn="$(frames) frames drawn; Weston $weston_version cannot capture the screen for a closer look"
        break
    fi
done
if [ -z "$drawn" ]; then
    if [ "$mode" = wayland ]; then
        fail "the window was not drawn: $(frames) frames, $colours colours on the screen"
    fi
    fail "the window was not drawn: $colours colours on the screen"
fi

# A program that cannot go on drawing exits within the first frames.
sleep 3
kill -0 "$app" 2>/dev/null || fail "the program exited after opening its window"

if [ -n "$screenshot" ] && [ -s "$shot" ]; then
    cp "$shot" "$screenshot"
fi
echo "smoke test ($mode) passed: the window is open, $drawn"
