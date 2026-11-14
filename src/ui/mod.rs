//! Interface logic that does not depend on a window: board geometry, choosing a move
//! with the mouse, animation and effect timing, the pictures of a piece and of the
//! application icon, and the game session. Drawing on the screen is in the executable.

pub mod anim;
pub mod fx;
pub mod geometry;
pub mod icon;
pub mod input;
pub mod session;
pub mod sprite;
