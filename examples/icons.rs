//! Writes the icon files of the application into `assets/icons/`.
//!
//! The icon is drawn by the program itself (`rucheckers::ui::icon`). The Windows
//! executable and the desktop entry on Linux need it as a file, so the files are kept in
//! the repository. Run this from the root of the repository after changing the drawing
//! of a piece or of the icon:
//!
//! ```text
//! cargo run --example icons
//! ```

use rucheckers::ui::icon::{ico_file, png_file};
use std::fs;
use std::io;
use std::path::Path;

fn main() -> io::Result<()> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets").join("icons");
    fs::create_dir_all(&dir)?;
    for (name, bytes) in [("rucheckers.ico", ico_file()), ("rucheckers.png", png_file())] {
        let path = dir.join(name);
        fs::write(&path, &bytes)?;
        println!("{} ({} bytes)", path.display(), bytes.len());
    }
    Ok(())
}
