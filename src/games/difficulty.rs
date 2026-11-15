//! The shared difficulty scale; each game chooses its own search parameters.

pub const MIN_LEVEL: u8 = 1;
pub const MAX_LEVEL: u8 = 10;

#[must_use]
pub fn clamp_level(level: u8) -> u8 {
    level.clamp(MIN_LEVEL, MAX_LEVEL)
}
