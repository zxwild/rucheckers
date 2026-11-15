//! Board games: Russian draughts and Ugolki, independent engines and shared controllers.
//!
//! Nothing in this library depends on a window or a sound device, and all of it is
//! checked by `cargo test`. Drawing and sound playback live in the executable
//! (`src/main.rs`, `src/gui/`).

pub mod games;
// Keep the established Russian-draughts library API available to callers.
pub use games::russian::{ai, board, moves, notation, pdn, rules, tree};
pub mod i18n;
pub mod rng;
pub mod settings;
pub mod sound;
pub mod ui;
