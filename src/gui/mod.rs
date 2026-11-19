//! Graphical interface: the window, board drawing, side panel, dialogs and sound.

mod app;
mod audio;
mod board;
mod files;
mod fonts;
mod icon;
mod theme;
mod window;

use eframe::egui;
use rucheckers::i18n::{tr, Lang, Text};
use rucheckers::settings::{default_settings_path, system_locale, Settings, MIN_WINDOW_SIZE};
use std::ffi::OsStr;
use std::path::PathBuf;
use std::sync::Arc;

/// The language of the user's account as the operating system reports it.
///
/// Windows and macOS do not put the language into environment variables for programs
/// started from the desktop, so the system is asked directly.
#[cfg(any(windows, target_os = "macos"))]
fn account_locale() -> Option<String> {
    sys_locale::get_locale()
}

/// On Linux the environment variables are the system's way to name the language.
#[cfg(not(any(windows, target_os = "macos")))]
const fn account_locale() -> Option<String> {
    None
}

/// Runs the application and returns after the window is closed.
///
/// # Errors
///
/// Returns the error of the windowing or graphics system if the window cannot be
/// opened. The user has been told about it by then.
pub fn run() -> eframe::Result<()> {
    let settings_path = default_settings_path();
    // Environment variables come first on every system: they are how a user overrides
    // the language for one program.
    let locale = system_locale().or_else(account_locale);
    let base = Settings::for_locale(locale.as_deref());
    let settings = match &settings_path {
        Some(path) => Settings::load(path, base),
        None => base,
    };
    let lang = settings.lang;

    // An optional command line argument is a game file to open.
    let initial_file = std::env::args_os()
        .nth(1)
        .filter(|arg| !(cfg!(target_os = "macos") && is_process_serial_number(arg)))
        .map(PathBuf::from);

    let viewport = egui::ViewportBuilder::default()
        .with_title(tr(lang, settings.game.text_key()))
        .with_app_id("rucheckers")
        .with_min_inner_size(MIN_WINDOW_SIZE.map(|side| side as f32))
        .with_icon(Arc::new(icon::app_icon()));
    let options = eframe::NativeOptions {
        viewport: window::restore(viewport, &settings.window),
        ..Default::default()
    };
    eframe::run_native(
        "rucheckers",
        options,
        Box::new(move |cc| {
            Ok(Box::new(app::CheckersApp::new(
                cc,
                settings,
                settings_path,
                initial_file,
            )))
        }),
    )
    .inspect_err(|error| report_failure(lang, error))
}

/// Whether an argument is the process serial number that older versions of macOS pass
/// to an application started from Finder (`-psn_0_12345`). It is not a file to open.
fn is_process_serial_number(arg: &OsStr) -> bool {
    arg.as_encoded_bytes().starts_with(b"-psn_")
}

/// Tells the user why the program could not start.
fn report_failure(lang: Lang, error: &eframe::Error) {
    let message = format!("{}\n\n{error}", tr(lang, Text::StartFailed));
    eprintln!("rucheckers: {message}");
    // A windowed program on Windows has no console to print to.
    #[cfg(windows)]
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title(tr(lang, Text::AppTitle))
        .set_description(message)
        .show();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_process_serial_number_is_not_a_file() {
        assert!(is_process_serial_number(OsStr::new("-psn_0_12345")));
        for file in ["game.pdn", "/Users/me/-psn_0_1.pdn", "-psn", "psn_0_1", ""] {
            assert!(!is_process_serial_number(OsStr::new(file)), "{file}");
        }
    }
}
