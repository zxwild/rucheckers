//! Русские шашки: правила, компьютерный соперник, дерево партии, PDN и логика интерфейса.
//!
//! Всё, что находится в этой библиотеке, не зависит от окна и звукового устройства
//! и проверяется командой `cargo test`. Отрисовка и воспроизведение звука живут
//! в исполняемом файле (`src/main.rs`, `src/gui/`).

pub mod ai;
pub mod board;
pub mod i18n;
pub mod moves;
pub mod notation;
pub mod pdn;
pub mod rng;
pub mod rules;
pub mod settings;
pub mod sound;
pub mod tree;
pub mod ui;
