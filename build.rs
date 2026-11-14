//! Build script: gives the Windows executable its icon, version information and
//! manifest. For other systems it does nothing.

use std::env;

/// The icon shown by Explorer, the Start menu and pinned shortcuts. It is written by
/// `cargo run --example icons`.
const ICON: &str = "assets/icons/rucheckers.ico";
/// The application manifest: Windows 10 and 11, sharp drawing on dense screens.
const MANIFEST: &str = "assets/windows/rucheckers.manifest";
/// The name Windows shows for the program in the Task Manager and in "Open with".
const TITLE: &str = "Russian Draughts";
/// The language of the texts above: English (United States).
const LANGUAGE: u16 = 0x0409;

fn main() {
    for input in ["build.rs", ICON, MANIFEST] {
        println!("cargo:rerun-if-changed={input}");
    }
    // The script runs on the building machine; the system it builds for is in the
    // environment.
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut resources = winresource::WindowsResource::new();
    resources
        .set_icon(ICON)
        .set_manifest_file(MANIFEST)
        .set_language(LANGUAGE)
        .set("ProductName", TITLE)
        .set("FileDescription", TITLE)
        .set("OriginalFilename", "rucheckers.exe");
    // The resources need the resource compiler of the toolchain (`rc.exe` of the Windows
    // SDK or `windres` of MinGW). Without it the program still builds and works; only
    // the file in Explorer has no icon. The window takes its icon from the program.
    if let Err(error) = resources.compile() {
        println!("cargo:warning=the executable is built without its icon and manifest: {error}");
    }
}
