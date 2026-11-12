//! Графический интерфейс: окно, отрисовка доски, панель ходов, диалоги и звук.

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

/// Запускает приложение и возвращается после закрытия окна.
pub fn run() -> eframe::Result<()> {
    let settings_path = default_settings_path();
    let base = Settings::for_locale(system_locale().as_deref());
    let settings = match &settings_path {
        Some(path) => Settings::load(path, base),
        None => base,
    };

    // Необязательный аргумент командной строки — файл партии, который нужно открыть.
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
