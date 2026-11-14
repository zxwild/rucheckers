//! Reference for the move generator: the number of move sequences from the starting position.
//!
//! The numbers were verified independently. If the test fails, the generator is wrong;
//! the numbers must not be changed.

use rucheckers::board::Position;
use rucheckers::moves::perft;

/// Depths 1–9. Captures with different routes count as different moves.
const EXPECTED: [u64; 9] = [7, 49, 302, 1469, 7482, 37986, 190_146, 929_905, 4_570_667];

#[test]
fn perft_from_initial_position_depths_1_to_9() {
    let pos = Position::initial();
    for (i, &expected) in EXPECTED.iter().enumerate() {
        let depth = i as u32 + 1;
        assert_eq!(perft(&pos, depth), expected, "perft({depth})");
    }
}
