//! Static evaluation of a position.
//!
//! The score is measured in hundredths of a man and is made up of material, the
//! advancement and placement of men, the guard of the promotion squares, the mobility
//! of kings, and corrections for endgames in which an advantage cannot be converted.

use crate::board::{squares_of, Color, Position, Square, MAIN_ROAD, NEIGHBOR, NO_SQUARE};

/// Value of a man.
pub const MAN_VALUE: i32 = 100;
/// Value of a king.
pub const KING_VALUE: i32 = 290;

/// Bonus of a man for advancing along the ranks (from its own side's point of view).
const ADVANCE: [i32; 8] = [0, 2, 4, 7, 10, 14, 20, 0];
/// Bonus of a man for being close to the centre along the files.
const FILE_BONUS: [i32; 8] = [-4, 0, 3, 5, 5, 3, 0, -4];
/// Bonus of men on their own back rank: they keep the opponent from promoting.
/// The index is the number of the square (a1, c1, e1, g1) from the side's point of view.
const BACK_RANK_BONUS: [i32; 4] = [2, 8, 8, 4];

const fn build_man_table() -> [i32; 32] {
    let mut table = [0; 32];
    let mut i = 0;
    while i < 32 {
        let rank = i / 4;
        let file = 2 * (i % 4) + rank % 2;
        table[i] = FILE_BONUS[file];
        // The central squares c3–f6 are worth more than the rest.
        if rank >= 2 && rank <= 5 && file >= 2 && file <= 5 {
            table[i] += 3;
        }
        i += 1;
    }
    table
}

/// Positional value of a square for a white man; for a black one the square is mirrored (31 − i).
static MAN_TABLE: [i32; 32] = build_man_table();

/// Number of empty squares a king can reach with a quiet move.
#[allow(clippy::needless_range_loop)]
fn king_mobility(sq: Square, occupied: u32) -> i32 {
    let mut count = 0;
    for dir in 0..4 {
        let mut cur = NEIGHBOR[sq.index()][dir];
        while cur != NO_SQUARE && occupied & (1 << cur) == 0 {
            count += 1;
            cur = NEIGHBOR[cur as usize][dir];
        }
    }
    count
}

/// Score from White's point of view.
#[must_use]
pub fn evaluate_white(pos: &Position) -> i32 {
    let white_men = pos.men(Color::White);
    let black_men = pos.men(Color::Black);
    let white_kings = pos.kings_of(Color::White);
    let black_kings = pos.kings_of(Color::Black);
    let (wm, bm) = (white_men.count_ones() as i32, black_men.count_ones() as i32);
    let (wk, bk) = (white_kings.count_ones() as i32, black_kings.count_ones() as i32);
    let total = wm + bm + wk + bk;
    if total == 0 {
        return 0;
    }
    let occupied = pos.occupied();

    // Material. The fewer pieces on the board, the heavier an advantage weighs: this
    // encourages the stronger side to exchange.
    let material = MAN_VALUE * (wm - bm) + KING_VALUE * (wk - bk);
    let mut score = material + material * (24 - total).max(0) / 48;

    // Men: placement and advancement. Towards the end of the game advancement matters more.
    let advance_weight = 16 + (24 - total).max(0);
    let mut placement = 0;
    let mut advance = 0;
    for sq in squares_of(white_men) {
        placement += MAN_TABLE[sq.index()];
        advance += ADVANCE[sq.rank() as usize];
    }
    for sq in squares_of(black_men) {
        placement -= MAN_TABLE[31 - sq.index()];
        advance -= ADVANCE[7 - sq.rank() as usize];
    }
    score += placement + advance * advance_weight / 16;

    // Guard of the promotion squares while the opponent still has men.
    if bm > 0 {
        let weight = bm.min(6);
        let mut guard = 0;
        for sq in squares_of(white_men & 0x0000_000F) {
            guard += BACK_RANK_BONUS[sq.index()];
        }
        score += guard * weight / 6;
    }
    if wm > 0 {
        let weight = wm.min(6);
        let mut guard = 0;
        for sq in squares_of(black_men & 0xF000_0000) {
            guard += BACK_RANK_BONUS[31 - sq.index()];
        }
        score -= guard * weight / 6;
    }

    // Kings: mobility and control of the main road.
    for sq in squares_of(white_kings) {
        score += king_mobility(sq, occupied);
        if sq.on_main_road() {
            score += 10;
        }
    }
    for sq in squares_of(black_kings) {
        score -= king_mobility(sq, occupied);
        if sq.on_main_road() {
            score -= 10;
        }
    }

    // Endgames in which a material advantage usually cannot be converted.
    if wk > 0 && bk > 0 {
        let (strong_total, weak, weak_total) = if score >= 0 {
            (wm + wk, Color::Black, bm + bk)
        } else {
            (bm + bk, Color::White, wm + wk)
        };
        if weak_total == 1 {
            // A lone king: two pieces cannot catch it, three only if it is off the main road.
            let on_road = pos.kings_of(weak) & MAIN_ROAD != 0;
            if strong_total <= 2 || (strong_total == 3 && on_road) {
                score /= 8;
            }
        } else if wm == 0 && bm == 0 && (wk - bk).abs() <= 1 {
            // Kings only, in almost equal numbers.
            score /= 3;
        }
    }

    score
}

/// Score from the point of view of the side to move.
#[inline]
#[must_use]
pub fn evaluate(pos: &Position) -> i32 {
    let score = evaluate_white(pos);
    match pos.side {
        Color::White => score,
        Color::Black => -score,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(fen: &str) -> i32 {
        evaluate_white(&Position::from_fen(fen).unwrap())
    }

    /// The position mirrored through the centre of the board, with the colours swapped.
    fn mirrored(pos: &Position) -> Position {
        Position {
            white: pos.black.reverse_bits(),
            black: pos.white.reverse_bits(),
            kings: pos.kings.reverse_bits(),
            side: pos.side.opponent(),
        }
    }

    #[test]
    fn initial_position_is_balanced() {
        assert_eq!(evaluate_white(&Position::initial()), 0);
        assert_eq!(evaluate(&Position::initial()), 0);
    }

    #[test]
    fn evaluation_is_symmetric() {
        for fen in [
            "W:Wa1,c1,e3,g3,d4:Bb8,d8,f6,h6,c5",
            "B:WKa1,c3,e3:BKh8,f6,b6,d6",
            "W:Wc3,Kd8:Bf6,Kg1",
            "W:WKa1,Kc1,Ke1:BKh8",
            "B:Wa3,b2:Bh6,g7,f8",
        ] {
            let pos = Position::from_fen(fen).unwrap();
            let mirror = mirrored(&pos);
            assert!(mirror.is_valid(), "{fen}");
            assert_eq!(evaluate_white(&pos), -evaluate_white(&mirror), "{fen}");
            assert_eq!(evaluate(&pos), evaluate(&mirror), "{fen}");
        }
    }

    #[test]
    fn material_matters() {
        assert!(eval("W:Wa1,c1,e1:Bb8,d8") > 50);
        assert!(eval("W:Wa1,c1:Bb8,d8,f8") < -50);
        assert!(
            eval("W:WKa1,c1:Bb8,d8") > 150,
            "a king is clearly stronger than a man"
        );
        assert!(eval("W:Wc3,e3,g3,a3,b2:Bb6,d6,f6,h6") > eval("W:Wc3,e3,g3,a3:Bb6,d6,f6,h6"));
    }

    #[test]
    fn lone_king_on_the_main_road_is_drawish() {
        let on_road = eval("W:WKc1,Ke1,Kg1:BKd4");
        let off_road = eval("W:WKc1,Ke1,Kg1:BKb4");
        assert!(off_road > 4 * on_road, "{off_road} versus {on_road}");
        assert!(eval("W:WKc1,Ke1:BKb4") < 150, "two kings cannot catch one");
    }

    #[test]
    fn advanced_men_are_worth_more() {
        assert!(eval("W:We5:Bh8") > eval("W:We3:Bh8"));
        assert!(eval("W:Wd6:Bh8") > eval("W:Wd4:Bh8"));
    }
}
