//! The window icon: drawn by the program (`rucheckers::ui::icon`).

use eframe::egui::IconData;
use rucheckers::ui::icon::render_icon;

/// Side of the window icon in pixels. The system scales it down for title bars, docks
/// and window switchers.
const SIZE: usize = 128;

/// The application icon: a white piece on a dark square, the same one as on the board.
pub fn app_icon() -> IconData {
    IconData {
        rgba: render_icon(SIZE).rgba,
        width: SIZE as u32,
        height: SIZE as u32,
    }
}
