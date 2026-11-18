#!/bin/sh
# Builds the program as a macOS application: target/macos/Russian Draughts.app, a bundle
# with the executable, the icon, the description the system reads (Info.plist) and the
# name of the program in every language of the interface. Finder, the Dock, Launchpad
# and Spotlight then show it like any other application.
#
#   tools/build-macos-app.sh                builds the bundle for the processor of this Mac
#   tools/build-macos-app.sh --universal    for Apple silicon and Intel processors at once
#   tools/build-macos-app.sh --dmg          also packs the bundle into a disk image
#   tools/build-macos-app.sh --install      also copies it into /Applications
#   tools/build-macos-app.sh --uninstall    removes it from there (same APPLICATIONS)
#
# The first three options can be combined. The environment can change:
#
#   APPLICATIONS              where --install puts the bundle: /Applications by default,
#                             ~/Applications for the current user only
#   CODESIGN_IDENTITY         the certificate to sign with, such as "Developer ID
#                             Application: ..."; without it the bundle is signed ad hoc,
#                             which is enough on the Mac it was built on
#   MACOSX_DEPLOYMENT_TARGET  the oldest version of macOS the program runs on, 11.0 by
#                             default
#
# Needs Rust and the Xcode command line tools (xcode-select --install).
set -eu

root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
name="Russian Draughts"
target_dir=${CARGO_TARGET_DIR:-"$root/target"}
out="$target_dir/macos"
app="$out/$name.app"
applications=${APPLICATIONS:-/Applications}
installed="$applications/$name.app"
identity=${CODESIGN_IDENTITY:--}
MACOSX_DEPLOYMENT_TARGET=${MACOSX_DEPLOYMENT_TARGET:-11.0}
export MACOSX_DEPLOYMENT_TARGET
# The database of Launch Services: which applications there are and what their icons are.
lsregister=/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister

usage() {
    echo "usage: $0 [--universal] [--dmg] [--install] | --uninstall" >&2
    exit 2
}

fail() {
    echo "$0: $1" >&2
    exit 1
}

need() {
    command -v "$1" >/dev/null 2>&1 || fail "$1 is missing: $2"
}

register() {
    if [ -x "$lsregister" ]; then
        "$lsregister" "$@" || true
    fi
}

universal=
dmg=
install=
uninstall=
for arg in "$@"; do
    case "$arg" in
    --universal) universal=1 ;;
    --dmg) dmg=1 ;;
    --install) install=1 ;;
    --uninstall) uninstall=1 ;;
    *) usage ;;
    esac
done
if [ -n "$uninstall" ] && [ -n "$universal$dmg$install" ]; then
    usage
fi
[ "$(uname -s)" = Darwin ] || fail "a macOS application is built on macOS"

if [ -n "$uninstall" ]; then
    # The settings stay in ~/Library/Application Support/rucheckers.
    if [ -d "$installed" ]; then
        register -u "$installed"
        rm -rf -- "$installed"
    fi
    echo "Removed: $installed"
    exit 0
fi

tools="install the Xcode command line tools: xcode-select --install"
need cargo "install Rust: https://rustup.rs"
for tool in codesign ditto iconutil lipo plutil sips; do
    need "$tool" "$tools"
done

# The version of the program. macOS takes up to three numbers, so a pre-release suffix
# such as -beta.1 is left out.
version=$(awk -F'"' '
    /^\[/ { section = $0 }
    section == "[package]" && /^version[ \t]*=/ { print $2; exit }
' "$root/Cargo.toml")
[ -n "$version" ] || fail "Cargo.toml has no version"
bundle_version=${version%%[-+]*}

# The executable. A universal one holds a build for each processor.
if [ -n "$universal" ]; then
    need rustup "--universal adds the standard library of the other processor with rustup"
    rustup target add aarch64-apple-darwin x86_64-apple-darwin
    for target in aarch64-apple-darwin x86_64-apple-darwin; do
        cargo build --release --manifest-path "$root/Cargo.toml" --target "$target"
    done
    mkdir -p "$out"
    executable="$out/rucheckers-universal"
    lipo -create -output "$executable" \
        "$target_dir/aarch64-apple-darwin/release/rucheckers" \
        "$target_dir/x86_64-apple-darwin/release/rucheckers"
    arch=universal
    # The icons are drawn by a build for this Mac, which one of the two already is.
    host=$(rustc -vV | sed -n 's/^host: //p')
    example_target="--target=$host"
else
    cargo build --release --manifest-path "$root/Cargo.toml"
    executable="$target_dir/release/rucheckers"
    arch=$(lipo -archs "$executable")
    example_target=
fi

# The icon. The program draws it at every size macOS asks for; iconutil packs the images.
# The program writes them uncompressed, sips compresses them.
iconset="$out/rucheckers.iconset"
rm -rf -- "$iconset"
# shellcheck disable=SC2086 # example_target is empty or a single word
cargo run --release --quiet --manifest-path "$root/Cargo.toml" $example_target \
    --example icons -- --iconset "$iconset" >/dev/null
for image in "$iconset"/*.png; do
    sips -s format png "$image" --out "$image" >/dev/null
done

# The bundle.
rm -rf -- "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp -X "$executable" "$app/Contents/MacOS/$name"
sed -e "s/@VERSION@/$bundle_version/g" \
    -e "s/@MINIMUM_SYSTEM_VERSION@/$MACOSX_DEPLOYMENT_TARGET/g" \
    "$root/assets/macos/Info.plist" >"$app/Contents/Info.plist"
plutil -lint -s "$app/Contents/Info.plist"
printf 'APPL????' >"$app/Contents/PkgInfo"
iconutil -c icns -o "$app/Contents/Resources/rucheckers.icns" "$iconset"
# The name in every language, in the binary form the system reads fastest.
for lproj in "$root"/assets/macos/*.lproj; do
    dest="$app/Contents/Resources/$(basename -- "$lproj")"
    mkdir -p "$dest"
    plutil -convert binary1 -o "$dest/InfoPlist.strings" "$lproj/InfoPlist.strings"
done

# The signature seals the bundle: macOS does not start a changed one. The hardened
# runtime and the secure timestamp are what notarization by Apple requires.
if [ "$identity" = - ]; then
    codesign --force --sign - "$app"
else
    codesign --force --sign "$identity" --options runtime --timestamp "$app"
fi
codesign --verify --strict "$app"
echo "Built: $app ($arch)"

# A disk image with the bundle and a link to /Applications to drag it onto.
if [ -n "$dmg" ]; then
    need hdiutil "$tools"
    image="$out/rucheckers-$version-macos-$arch.dmg"
    staging="$out/dmg"
    rm -rf -- "$staging"
    mkdir -p "$staging"
    ditto "$app" "$staging/$name.app"
    ln -s /Applications "$staging/Applications"
    # hdiutil now and then finds a disk busy right after a build: it gets three tries.
    tries=1
    until hdiutil create -quiet -ov -volname "$name" -srcfolder "$staging" \
        -fs HFS+ -format UDZO "$image"; do
        [ "$tries" -lt 3 ] || fail "hdiutil could not create $image"
        tries=$((tries + 1))
        sleep 5
    done
    rm -rf -- "$staging"
    if [ "$identity" != - ]; then
        codesign --force --sign "$identity" --timestamp "$image"
    fi
    echo "Disk image: $image"
fi

if [ -n "$install" ]; then
    mkdir -p "$applications" 2>/dev/null || true
    if ! [ -d "$applications" ] || ! [ -w "$applications" ]; then
        fail "$applications is not writable; APPLICATIONS=~/Applications installs for the current user"
    fi
    rm -rf -- "$installed"
    ditto "$app" "$installed"
    # A new modification time and registration make Finder and the Dock show the new icon.
    touch "$installed"
    register -f "$installed"
    echo "Installed: $installed"
fi
