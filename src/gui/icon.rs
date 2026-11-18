//! The window icon: drawn by the program (`rucheckers::ui::icon`).
//!
//! On macOS the icon of a running program is the one in the Dock. When the program runs
//! from an application bundle (`tools/build-macos-app.sh`), the Dock already shows the
//! `.icns` file of the bundle, which the system draws at every size; the window then
//! gives no icon, so that the Dock keeps it. A program started outside a bundle, such
//! as by `cargo run`, has no icon of its own and sets the macOS variant of the icon.

use eframe::egui::IconData;
#[cfg(not(target_os = "macos"))]
use rucheckers::ui::icon::render_icon;
#[cfg(target_os = "macos")]
use rucheckers::ui::icon::render_macos_icon;
use rucheckers::ui::icon::IconImage;
use std::path::Path;

/// The application icon: a white piece on a dark square, the same one as on the board.
///
/// Side of the icon in pixels: the system scales it down for title bars, docks and
/// window switchers. The Dock of macOS shows icons larger than other systems do.
#[cfg(not(target_os = "macos"))]
pub fn app_icon() -> IconData {
    icon_data(render_icon(128))
}

/// The application icon for macOS. See the module documentation.
#[cfg(target_os = "macos")]
pub fn app_icon() -> IconData {
    let bundled = std::env::current_exe().is_ok_and(|exe| inside_app_bundle(&exe));
    if bundled {
        // An empty icon: eframe then leaves the icon of the Dock alone, while without
        // an icon at all it would set the logo of egui.
        IconData::default()
    } else {
        icon_data(render_macos_icon(256))
    }
}

fn icon_data(image: IconImage) -> IconData {
    let side = u32::try_from(image.size).expect("the icon is small");
    IconData {
        rgba: image.rgba,
        width: side,
        height: side,
    }
}

/// Whether an executable lies where macOS keeps the executable of an application bundle:
/// `Name.app/Contents/MacOS/`.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn inside_app_bundle(exe: &Path) -> bool {
    let Some(macos) = exe.parent() else {
        return false;
    };
    let Some(contents) = macos.parent() else {
        return false;
    };
    let Some(bundle) = contents.parent() else {
        return false;
    };
    macos.file_name().is_some_and(|name| name == "MacOS")
        && contents.file_name().is_some_and(|name| name == "Contents")
        && bundle
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("app"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_executable_inside_a_bundle_is_recognised() {
        for exe in [
            "/Applications/Russian Draughts.app/Contents/MacOS/Russian Draughts",
            "/Users/me/Applications/Russian Draughts.app/Contents/MacOS/Russian Draughts",
            "target/macos/Russian Draughts.app/Contents/MacOS/Russian Draughts",
            "/Volumes/Russian Draughts/Russian Draughts.APP/Contents/MacOS/rucheckers",
        ] {
            assert!(inside_app_bundle(Path::new(exe)), "{exe}");
        }
    }

    #[test]
    fn other_executables_are_not_in_a_bundle() {
        for exe in [
            "/Users/me/rucheckers/target/release/rucheckers",
            "/usr/local/bin/rucheckers",
            "/Applications/Russian Draughts.app/Contents/Resources/rucheckers",
            "/Applications/Russian Draughts/Contents/MacOS/rucheckers",
            "/Applications/Russian Draughts.app/MacOS/rucheckers",
            "Contents/MacOS/rucheckers",
            "MacOS/rucheckers",
            "rucheckers",
            "",
        ] {
            assert!(!inside_app_bundle(Path::new(exe)), "{exe}");
        }
    }

    #[test]
    fn the_icon_is_square_and_complete() {
        let icon = app_icon();
        assert_eq!(icon.width, icon.height);
        assert_eq!(icon.rgba.len(), (icon.width * icon.height * 4) as usize);
        assert!(icon.width >= 128);
    }
}
