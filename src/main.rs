//! Russian draughts: a game against the computer. Entry point of the desktop application.
//!
//! All game logic is in the `rucheckers` library and is tested without a window; here
//! there is only the window, drawing, sound and system dialogs.

// A release build for Windows is a windowed program: it opens no console next to the
// window. Debug builds keep the console for diagnostic output.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod gui;

use std::process::ExitCode;

fn main() -> ExitCode {
    // A failure has already been reported to the user in their language.
    match gui::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}
