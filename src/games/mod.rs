//! Common board presentation and controllers, with independent game engines.

pub mod anim;
pub mod board;
pub mod difficulty;
pub mod input;
mod kind;
pub mod moves;
pub mod notation;
pub mod pieces;
mod presentation;
pub mod record;
pub mod russian;
pub mod session;
mod state;
pub mod tree;
pub mod ugolki;
pub mod worker;

pub use board::{squares_of, Position, Square};
pub use kind::GameKind;
pub use moves::Move;
pub use notation::{format_move, parse_move};
pub use pieces::{Color, Kind, Piece};
pub use presentation::{outcome_reason, outcome_title};
pub use state::{choose_move, legal_moves, FinishReason, GameState, Outcome};
