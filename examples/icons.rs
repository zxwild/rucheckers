//! Writes the icon files of the application.
//!
//! The icon is drawn by the program itself (`rucheckers::ui::icon`). The Windows
//! executable and the desktop entry on Linux need it as a file, so the files are kept in
//! the repository, in `assets/icons/`. Run this from the root of the repository after
//! changing the drawing of a piece or of the icon:
//!
//! ```text
//! cargo run --example icons
//! ```
//!
//! With `--iconset DIR` it writes the images of the macOS icon instead, under the names
//! `iconutil` expects. `tools/build-macos-app.sh` packs them into the `.icns` file of the
//! application bundle, so they are not kept in the repository:
//!
//! ```text
//! cargo run --example icons -- --iconset target/macos/rucheckers.iconset
//! ```

use rucheckers::ui::icon::{encode_png, ico_file, png_file, render_macos_icon, ICONSET};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1);
    let result = match (args.next(), args.next(), args.next()) {
        (None, ..) => repository_icons(),
        (Some(flag), Some(dir), None) if flag == "--iconset" => iconset(&PathBuf::from(dir)),
        _ => {
            eprintln!("usage: cargo run --example icons [-- --iconset DIR]");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("icons: {error}");
            ExitCode::FAILURE
        }
    }
}

/// The icons kept in the repository: for Windows and for Linux.
fn repository_icons() -> io::Result<()> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets").join("icons");
    fs::create_dir_all(&dir)?;
    for (name, bytes) in [("rucheckers.ico", ico_file()), ("rucheckers.png", png_file())] {
        write(&dir.join(name), &bytes)?;
    }
    Ok(())
}

/// The images of the macOS icon, a `.iconset` directory for `iconutil`.
fn iconset(dir: &Path) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    for (name, size) in ICONSET {
        write(&dir.join(name), &encode_png(&render_macos_icon(size)))?;
    }
    Ok(())
}

fn write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    fs::write(path, bytes)?;
    println!("{} ({} bytes)", path.display(), bytes.len());
    Ok(())
}
