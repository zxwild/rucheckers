//! Graphical interface: the window, board drawing, move panel, dialogs and sound.

mod app;
mod audio;
mod board;
mod files;
mod fonts;
mod icon;
mod theme;

use eframe::egui;
use rucheckers::i18n::{tr, Text};
use rucheckers::settings::{default_settings_path, system_locale, Settings};
use std::path::PathBuf;
use std::sync::Arc;

/// Runs the application and returns after the window is closed.
pub fn run() -> eframe::Result<()> {
    let settings_path = default_settings_path();
    let base = Settings::for_locale(system_locale().as_deref());
    let settings = match &settings_path {
        Some(path) => Settings::load(path, base),
        None => base,
    };

    // An optional command line argument is a game file to open.
    let initial_file = std::env::args_os().nth(1).map(PathBuf::from);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(tr(settings.lang, Text::AppTitle))
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
}
