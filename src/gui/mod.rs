//! Graphical interface: the window, board drawing, move panel, dialogs and sound.

mod app;
mod audio;
mod board;
mod files;
mod fonts;
mod icon;
mod theme;

use eframe::egui;
use rucheckers::i18n::{tr, Lang, Text};
use rucheckers::settings::{default_settings_path, system_locale, Settings};
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
    let initial_file = std::env::args_os().nth(1).map(PathBuf::from);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(tr(lang, Text::AppTitle))
            .with_app_id("rucheckers")
            .with_inner_size([1120.0, 760.0])
            .with_min_inner_size([440.0, 380.0])
            .with_icon(Arc::new(icon::app_icon())),
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
