//! Russian draughts: rules, computer opponent, game tree, PDN and interface logic.
//!
//! Nothing in this library depends on a window or a sound device, and all of it is
//! checked by `cargo test`. Drawing and sound playback live in the executable
//! (`src/main.rs`, `src/gui/`).

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
