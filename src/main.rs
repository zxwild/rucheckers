//! Russian draughts: a game against the computer. Entry point of the desktop application.
//!
//! All game logic is in the `rucheckers` library and is tested without a window; here
//! there is only the window, drawing, sound and system dialogs.

mod gui;

fn main() {
    if let Err(error) = gui::run() {
        eprintln!("rucheckers: {error}");
        std::process::exit(1);
    }
}
