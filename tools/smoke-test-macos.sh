#!/bin/sh
# Starts the application bundle on macOS the way Finder does and checks that the program
# opens its window, draws into it and stays open. CI runs this on the bundle that
# tools/build-macos-app.sh builds, on Apple silicon and on Intel.
#
#   tools/smoke-test-macos.sh [path/to/Russian Draughts.app] [screenshot.png]
#
# Needs the Xcode command line tools: a small Swift program finds the window and counts
# the colours in its picture. An empty window is one flat colour, the board with its
# pieces has hundreds and more. A machine that does not let the terminal record the screen
# gives no picture; the window and its size decide alone there.
set -eu

root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
app=${1:-"$root/target/macos/Russian Draughts.app"}
screenshot=${2:-}
if [ $# -gt 2 ] || ! [ -d "$app/Contents/MacOS" ]; then
    echo "usage: $0 [path/to/Russian Draughts.app] [screenshot.png]" >&2
    exit 2
fi
app=$(CDPATH='' cd -- "$app" && pwd)

work=$(mktemp -d)
pid=
cleanup() {
    [ -z "$pid" ] || kill "$pid" 2>/dev/null || true
    rm -rf "$work"
}
trap cleanup EXIT

fail() {
    echo "smoke test (macOS) failed: $1" >&2
    if [ -s "$work/app.log" ]; then
        echo "--- program" >&2
        tail -n 40 "$work/app.log" | sed 's/^/    /' >&2
    fi
    exit 1
}

cat >"$work/probe.swift" <<'EOF'
// probe window PID: the largest window of the process on the screen, "id width height".
// probe colours FILE: the number of distinct colours in a picture.
import CoreGraphics
import Foundation
import ImageIO

let args = CommandLine.arguments
if args.count == 3 && args[1] == "window", let pid = Int32(args[2]) {
    let windows = CGWindowListCopyWindowInfo(.optionOnScreenOnly, kCGNullWindowID) as? [[String: Any]] ?? []
    var best: (id: Int, width: Int, height: Int)?
    for window in windows {
        guard (window[kCGWindowOwnerPID as String] as? NSNumber)?.int32Value == pid,
              (window[kCGWindowLayer as String] as? NSNumber)?.intValue == 0,
              let id = (window[kCGWindowNumber as String] as? NSNumber)?.intValue,
              let bounds = window[kCGWindowBounds as String] as? NSDictionary,
              let rect = CGRect(dictionaryRepresentation: bounds) else { continue }
        let (width, height) = (Int(rect.width), Int(rect.height))
        if width * height > (best.map { $0.width * $0.height } ?? 0) {
            best = (id, width, height)
        }
    }
    guard let found = best else { exit(1) }
    print(found.id, found.width, found.height)
} else if args.count == 3 && args[1] == "colours" {
    guard let source = CGImageSourceCreateWithURL(URL(fileURLWithPath: args[2]) as CFURL, nil),
          let image = CGImageSourceCreateImageAtIndex(source, 0, nil),
          let context = CGContext(data: nil, width: image.width, height: image.height,
                                  bitsPerComponent: 8, bytesPerRow: image.width * 4,
                                  space: CGColorSpaceCreateDeviceRGB(),
                                  bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue),
          let data = context.data else { exit(1) }
    context.draw(image, in: CGRect(x: 0, y: 0, width: image.width, height: image.height))
    let pixels = UnsafeBufferPointer(start: data.bindMemory(to: UInt32.self, capacity: image.width * image.height),
                                     count: image.width * image.height)
    print(Set(pixels).count)
} else {
    exit(2)
}
EOF
xcrun swiftc -O -o "$work/probe" "$work/probe.swift" || fail "the Swift probe did not compile"

# Launch Services starts the bundle: it reads Info.plist and checks the signature, as
# when the application is opened in Finder. A new instance is started each time.
executable=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$app/Contents/Info.plist")
: >"$work/app.log"
open -n --stdout "$work/app.log" --stderr "$work/app.log" "$app" || fail "open could not start the application"
for _ in 1 2 3 4 5 6 7 8 9 10; do
    pid=$(pgrep -n -f "$app/Contents/MacOS/$executable" || true)
    [ -z "$pid" ] || break
    sleep 1
done
[ -n "$pid" ] || fail "the program did not start"

shot="$work/window.png"
drawn=
window=
for _ in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20; do
    sleep 1
    kill -0 "$pid" 2>/dev/null || fail "the program exited"
    window=$("$work/probe" window "$pid") || continue
    read -r id width height <<EOF
$window
EOF
    if [ "$width" -lt 400 ] || [ "$height" -lt 300 ]; then
        continue
    fi
    if ! screencapture -x -o -l "$id" "$shot" 2>/dev/null || ! [ -s "$shot" ]; then
        drawn="a ${width}x$height window; the screen cannot be recorded here for a closer look"
        break
    fi
    colours=$("$work/probe" colours "$shot")
    if [ "$colours" -ge 256 ]; then
        drawn="a ${width}x$height window with $colours colours"
        break
    fi
done
[ -n "$window" ] || fail "the program opened no window"
[ -n "$drawn" ] || fail "the window was not drawn: ${colours:-no} colours in it"

# A program that cannot go on drawing exits within the first frames.
sleep 3
kill -0 "$pid" 2>/dev/null || fail "the program exited after opening its window"

if [ -n "$screenshot" ] && [ -s "$shot" ]; then
    cp "$shot" "$screenshot"
fi
echo "smoke test (macOS) passed: $drawn"
