//! Evaluation of a race position for the search.
//!
//! The main term is the work left to each side: the length of the shortest way to
//! move its pieces onto the cells of its goal, one piece to a cell. On this board the
//! goal is a rectangle in a corner, and the work splits into a shift along the ranks
//! and a shift along the files. Each shift is the earth mover's distance between the
//! pieces and the goal cells counted by file (or by rank), which is a sum over the gaps
//! between neighbouring files of the surplus of pieces on one side of the gap. This is
//! exact for nearly every position of a game and costs a few counts of bits; the change
//! made by a move is found in constant time from the signs of the surpluses.
//!
//! A convex price for distance far from the goal makes a lagging piece more urgent:
//! once the others have gone it has no pieces left to jump over. The side to move is
//! credited with the progress of its best move, so that positions reached after
//! different numbers of moves can be compared. Pieces that stay at home close to the
//! deadline of the home are expensive.

use super::bits::{adjacent, jump_closure, side_index, Board, Rules, FILE_A, RANK_1};
use super::{HomeSide, State, HOME_DEADLINE, PIECES};
use crate::games::{Color, Square};

/// Weights of the evaluation, chosen by matches of the program against itself.
#[derive(Clone, Copy, Debug)]
struct Weights {
    /// Price of one cell of the work.
    cell: i32,
    /// Extra price of the square of the distance beyond `far` cells from the corner of
    /// the goal.
    lag: i32,
    far: i32,
    /// Share of the best immediate progress credited to the side to move, in 1/16.
    initiative: i32,
    /// Price of one move still needed to leave the own home, per ply after
    /// `rush_from`.
    rush: i32,
    rush_from: u16,
    /// Price of each move by which the moves needed to leave the home exceed the moves
    /// left before the deadline, plus one: a side that needs all of them is in danger.
    crisis: i32,
}

const WEIGHTS: Weights = Weights {
    cell: 16,
    lag: 1,
    far: 4,
    initiative: 12,
    rush: 3,
    rush_from: 20,
    crisis: 200,
};

/// Most moves a piece is followed on its way out of its home; a piece that needs more
/// is treated as shut in.
const EXIT_HORIZON: i32 = 6;

/// How many moves the piece on `bit` needs to leave `home` if no other piece moved.
fn exit_moves(bit: u64, occupied: u64, home: u64) -> i32 {
    let rest = occupied & !bit;
    let mut frontier = bit;
    let mut seen = bit;
    for moves in 1..=EXIT_HORIZON {
        let reached = ((adjacent(frontier) & !rest) | jump_closure(frontier, rest)) & !seen;
        if reached & !home != 0 {
            return moves;
        }
        if reached == 0 {
            break;
        }
        seen |= reached;
        frontier = reached;
    }
    EXIT_HORIZON
}

/// Counts of the cells of a set on each file (axis 0) and on each rank (axis 1).
fn histogram(set: u64) -> [[i32; 8]; 2] {
    let mut counts = [[0; 8]; 2];
    for line in 0..8 {
        counts[0][line] = (set & (FILE_A << line)).count_ones() as i32;
        counts[1][line] = (set & (RANK_1 << (8 * line))).count_ones() as i32;
    }
    counts
}

/// The running surplus of pieces over goal cells on both sides of every gap between
/// neighbouring files and ranks: what the work of a side and its change by a move
/// depend on.
#[derive(Clone, Copy, Debug)]
pub struct Profile {
    /// The work in cells.
    pub work: i32,
    /// For each axis, how many of the gaps before a line have a surplus of pieces on
    /// the low side, and how many a shortage.
    surplus: [[i8; 8]; 2],
    shortage: [[i8; 8]; 2],
}

impl Profile {
    fn new(pieces: u64, goal: &[[i32; 8]; 2]) -> Self {
        let counts = histogram(pieces);
        let mut work = 0;
        let mut surplus = [[0; 8]; 2];
        let mut shortage = [[0; 8]; 2];
        for axis in 0..2 {
            let mut balance = 0;
            for gap in 0..7 {
                balance += counts[axis][gap] - goal[axis][gap];
                work += balance.abs();
                surplus[axis][gap + 1] = surplus[axis][gap] + i8::from(balance >= 1);
                shortage[axis][gap + 1] = shortage[axis][gap] + i8::from(balance <= -1);
            }
        }
        Self {
            work,
            surplus,
            shortage,
        }
    }

    /// Change of the work along one axis when a piece moves from line `a` to line `b`.
    fn shift(&self, axis: usize, a: usize, b: usize) -> i32 {
        if b > a {
            // The piece crosses the gaps a..b upwards: each gap with a surplus below it
            // gets one cell closer to balance, every other gap one cell further.
            (b - a) as i32 - 2 * i32::from(self.surplus[axis][b] - self.surplus[axis][a])
        } else {
            (a - b) as i32 - 2 * i32::from(self.shortage[axis][a] - self.shortage[axis][b])
        }
    }

    /// Change of the work in cells when a piece moves from `from` to `to`.
    #[must_use]
    pub fn change(&self, from: u8, to: u8) -> i32 {
        let (from, to) = (usize::from(from), usize::from(to));
        self.shift(0, from % 8, to % 8) + self.shift(1, from / 8, to / 8)
    }
}

#[derive(Clone, Debug)]
pub struct Evaluator {
    rules: Rules,
    weights: Weights,
    /// Goal cells by file and by rank for a side with `n` pieces: the deepest cells are
    /// treated as already filled by the pieces it does not have.
    goal: [[[[i32; 8]; 2]; PIECES + 1]; 2],
    /// Extra price of a piece of each side on each cell.
    lag: [[i32; 64]; 2],
    /// The side whose danger of failing the deadline of the home is left out. The side
    /// playing against it then gains nothing by blocking its way out of the home.
    spared: Option<Color>,
}

/// The cells of the goal of `color`, from the far corner outwards.
fn goal_by_depth(color: Color, rules: &Rules) -> Vec<(i32, i32, usize)> {
    let goal = rules.goal(color);
    let corner_file: u8 = if goal & FILE_A != 0 { 0 } else { 7 };
    let corner_rank: u8 = if color == Color::White { 7 } else { 0 };
    let mut cells: Vec<_> = Square::all()
        .filter(|square| goal & square.bit() != 0)
        .map(|square| {
            let rank = i32::from(square.rank().abs_diff(corner_rank));
            (
                i32::from(square.file().abs_diff(corner_file)) + rank,
                rank,
                square.index(),
            )
        })
        .collect();
    // Distance and rank distance identify a cell of the goal, the same way in both
    // layouts, so the order is the mirror image of the other layout's.
    cells.sort_unstable();
    cells
}

impl Evaluator {
    #[must_use]
    pub fn new(layout: HomeSide) -> Self {
        let weights = WEIGHTS;
        let rules = Rules::new(layout);
        let mut goal = [[[[0; 8]; 2]; PIECES + 1]; 2];
        let mut lag = [[0; 64]; 2];
        for color in [Color::White, Color::Black] {
            let side = side_index(color);
            let cells = goal_by_depth(color, &rules);
            for (count, histogram_of_open) in goal[side].iter_mut().enumerate() {
                // A side with fewer pieces than cells is finished when the shallowest
                // `count` cells are filled: the missing pieces stand in the deepest.
                let open = cells
                    .iter()
                    .skip(PIECES - count)
                    .fold(0, |mask, &(_, _, cell)| mask | (1u64 << cell));
                *histogram_of_open = histogram(open);
            }
            let corner = Square::from_index(cells[0].2).unwrap();
            for square in Square::all() {
                let distance =
                    i32::from(square.file().abs_diff(corner.file()) + square.rank().abs_diff(corner.rank()));
                let beyond = (distance - weights.far).max(0);
                lag[side][square.index()] = weights.lag * beyond * beyond;
            }
        }
        Self {
            rules,
            weights,
            goal,
            lag,
            spared: None,
        }
    }

    /// The same evaluation without the deadline danger of `spared`. The value stays the
    /// opposite for the two sides, so the search remains consistent.
    #[must_use]
    pub const fn sparing(mut self, spared: Option<Color>) -> Self {
        self.spared = spared;
        self
    }

    #[must_use]
    pub fn profile(&self, color: Color, pieces: u64) -> Profile {
        let count = (pieces.count_ones() as usize).min(PIECES);
        Profile::new(pieces, &self.goal[side_index(color)][count])
    }

    /// How much cheaper a move from `from` to `to` makes the pieces of `color`.
    #[must_use]
    pub fn progress(&self, color: Color, profile: &Profile, from: u8, to: u8) -> i32 {
        let lag = &self.lag[side_index(color)];
        lag[usize::from(from)] - lag[usize::from(to)] - self.weights.cell * profile.change(from, to)
    }

    /// The price of the pieces of `color`: their work and their lag.
    fn price(&self, color: Color, pieces: u64, profile: &Profile) -> i32 {
        let lag = &self.lag[side_index(color)];
        let mut total = profile.work * self.weights.cell;
        let mut rest = pieces;
        while rest != 0 {
            total += lag[rest.trailing_zeros() as usize];
            rest &= rest - 1;
        }
        total
    }

    /// The danger of failing the deadline of the home: the moves the pieces still at
    /// home need to get out, past the pieces around them, against the moves left.
    fn rush(&self, color: Color, pieces: u64, board: &Board) -> i32 {
        let ply = board.ply;
        let home = self.rules.home[side_index(color)];
        let mut staying = pieces & home;
        if staying == 0 || ply < self.weights.rush_from || ply >= HOME_DEADLINE * 2 {
            return 0;
        }
        if self.spared == Some(color) {
            return 0;
        }
        let occupied = board.occupied();
        let mut needed = 0;
        while staying != 0 {
            let bit = staying & staying.wrapping_neg();
            staying &= staying - 1;
            needed += exit_moves(bit, occupied, home);
        }
        // Moves of this side left before the check that follows Black's fortieth move.
        let next = if color == board.side { ply } else { ply + 1 };
        let left = (i32::from(HOME_DEADLINE * 2) - i32::from(next) + i32::from(color == Color::Black)) / 2;
        let late = i32::from(ply - self.weights.rush_from);
        needed * late * self.weights.rush + (needed - left + 1).max(0) * self.weights.crisis
    }

    /// The largest progress one move of the side to move can make, ignoring the rare
    /// moves forbidden by the lock rule and the deadline of the home.
    fn best_progress(&self, board: &Board, profile: &Profile) -> i32 {
        let occupied = board.occupied();
        let mut pieces = board.own();
        let mut best = 0;
        while pieces != 0 {
            let from = pieces.trailing_zeros() as u8;
            let bit = 1u64 << from;
            pieces &= pieces - 1;
            let mut targets = (adjacent(bit) & !occupied) | jump_closure(bit, occupied & !bit);
            while targets != 0 {
                let to = targets.trailing_zeros() as u8;
                targets &= targets - 1;
                best = best.max(self.progress(board.side, profile, from, to));
            }
        }
        best
    }

    /// The value of the position for the side to move.
    #[must_use]
    pub fn evaluate(&self, board: &Board) -> i32 {
        let me = board.side;
        let other = me.opponent();
        let mine = self.profile(me, board.own());
        let theirs = self.profile(other, board.other());
        let price = self.price(other, board.other(), &theirs) - self.price(me, board.own(), &mine);
        let rush = self.rush(other, board.other(), board) - self.rush(me, board.own(), board);
        let initiative = self.best_progress(board, &mine);
        price + rush + initiative * self.weights.initiative / 16
    }
}

/// The static value of `state` for `color`.
#[must_use]
pub fn evaluate(state: State, color: Color) -> i32 {
    let evaluator = Evaluator::new(state.pos.home_side);
    let board = Board {
        white: state.pos.white,
        black: state.pos.black,
        side: state.pos.side,
        ply: state.ply,
    };
    let value = evaluator.evaluate(&board);
    if color == state.pos.side {
        value
    } else {
        -value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::{squares_of, GameKind, Move, Position};
    use crate::rng::Rng;

    fn sq(name: &str) -> Square {
        Square::parse(name).unwrap()
    }

    fn state(fen: &str, ply: u16) -> State {
        State {
            pos: Position::from_fen_for(GameKind::Ugolki, fen).unwrap(),
            ply,
        }
    }

    fn board(state: State) -> Board {
        Board {
            white: state.pos.white,
            black: state.pos.black,
            side: state.pos.side,
            ply: state.ply,
        }
    }

    /// The cheapest matching of pieces to cells by Manhattan distance, by brute force
    /// over permutations of small sets.
    fn matching(pieces: &[Square], cells: &[Square]) -> i32 {
        fn go(pieces: &[Square], cells: &[Square], used: &mut Vec<bool>) -> i32 {
            let Some((&first, rest)) = pieces.split_first() else {
                return 0;
            };
            let mut best = i32::MAX;
            for (index, &cell) in cells.iter().enumerate() {
                if used[index] {
                    continue;
                }
                used[index] = true;
                let d = i32::from(first.file().abs_diff(cell.file()) + first.rank().abs_diff(cell.rank()));
                best = best.min(d + go(rest, cells, used));
                used[index] = false;
            }
            best
        }
        go(pieces, cells, &mut vec![false; cells.len()])
    }

    #[test]
    fn the_work_is_the_shortest_matching_and_its_change_is_exact() {
        let evaluator = Evaluator::new(HomeSide::Left);
        let rules = Rules::new(HomeSide::Left);
        let mut rng = Rng::new(0x77AB);
        let goal: Vec<_> = squares_of(rules.goal(Color::White)).collect();
        for _ in 0..300 {
            // Most of the pieces in the goal and a few outside, as at the end of a race.
            let mut cells: Vec<_> = Square::all().collect();
            rng.shuffle(&mut cells);
            let outside = 1 + rng.below(4);
            let mut inside = goal.clone();
            rng.shuffle(&mut inside);
            let mut pieces: Vec<_> = inside[..PIECES - outside].to_vec();
            pieces.extend(
                cells
                    .iter()
                    .filter(|sq| rules.goal(Color::White) & sq.bit() == 0)
                    .take(outside),
            );
            let mask = pieces.iter().fold(0, |mask, sq| mask | sq.bit());
            let open: Vec<_> = goal.iter().copied().filter(|sq| mask & sq.bit() == 0).collect();
            let outsiders: Vec<_> = pieces
                .iter()
                .copied()
                .filter(|sq| rules.goal(Color::White) & sq.bit() == 0)
                .collect();
            let profile = evaluator.profile(Color::White, mask);
            // Pieces already in the goal may have to make room, so the exact work is at
            // most the matching of the outsiders to the open cells.
            assert!(profile.work <= matching(&outsiders, &open));
            assert!(profile.work >= outsiders.len() as i32);
            for &from in &pieces {
                for to in Square::all().filter(|sq| mask & sq.bit() == 0) {
                    let moved = mask ^ from.bit() ^ to.bit();
                    let after = evaluator.profile(Color::White, moved).work;
                    assert_eq!(
                        profile.change(from.index() as u8, to.index() as u8),
                        after - profile.work
                    );
                }
            }
        }
        // From the start every assignment of pieces to cells is equally long.
        let start = rules.home[0];
        assert_eq!(evaluator.profile(Color::White, start).work, 108);
        assert_eq!(evaluator.profile(Color::White, rules.goal(Color::White)).work, 0);
    }

    #[test]
    fn the_last_piece_heads_for_the_open_cell_of_the_goal() {
        // Eleven pieces fill the goal except e6; the twelfth is on h5. Its way to e6 is
        // four cells, although it is closer to the far corner than e6 is.
        let rules = Rules::new(HomeSide::Left);
        let goal = rules.goal(Color::White) & !sq("e6").bit();
        let evaluator = Evaluator::new(HomeSide::Left);
        let profile = evaluator.profile(Color::White, goal | sq("h5").bit());
        assert_eq!(profile.work, 4);
        assert_eq!(profile.change(sq("h5").index() as u8, sq("g5").index() as u8), -1);
        assert_eq!(profile.change(sq("h5").index() as u8, sq("h4").index() as u8), 1);
    }

    #[test]
    fn a_stranded_rear_piece_matters_more_than_decorating_a_finished_corner() {
        let state = state("W:Wa1,h7:Be4", 60);
        let rescue = state.after(&Move::route(sq("a1"), &[sq("a2")]));
        let front = state.after(&Move::route(sq("h7"), &[sq("h8")]));
        assert!(evaluate(rescue, Color::White) > evaluate(front, Color::White));
    }

    #[test]
    fn the_back_of_the_goal_is_filled_before_its_entrance() {
        // Twelve pieces a side: the first white piece to arrive stands at the entrance
        // or in the far corner, everything else is the same.
        let rules = Rules::new(HomeSide::Left);
        let mut near = State::new(super::super::initial());
        near.pos.white = (rules.home[0] & !sq("a1").bit()) | sq("e6").bit();
        near.pos.black =
            (rules.home[1] & !sq("e6").bit() & !sq("h8").bit()) | sq("d4").bit() | sq("c4").bit();
        near.ply = 30;
        let mut deep = near;
        deep.pos.white = (rules.home[0] & !sq("a1").bit()) | sq("h8").bit();
        assert!(evaluate(deep, Color::White) > evaluate(near, Color::White));
    }

    #[test]
    fn the_way_out_of_the_home_counts_moves_past_other_pieces() {
        let home = Rules::new(HomeSide::Left).home[0];
        let bits = |names: &[&str]| names.iter().fold(0, |mask, name| mask | sq(name).bit());
        assert_eq!(exit_moves(sq("a1").bit(), bits(&["a1"]), home), 3);
        // A jump over a piece of either colour shortens the way.
        assert_eq!(exit_moves(sq("a1").bit(), bits(&["a1", "a2"]), home), 2);
        // d3 is boxed in by d4, d5, e3 and f3 and has to go round through c3.
        assert_eq!(
            exit_moves(sq("d3").bit(), bits(&["d3", "d4", "d5", "e3", "f3"]), home),
            2
        );
        assert_eq!(exit_moves(sq("d3").bit(), bits(&["d3"]), home), 1);
        // A piece that cannot move at all.
        let corner = bits(&["a1", "a2", "a3", "b1", "c1"]);
        assert_eq!(exit_moves(sq("a1").bit(), corner, home), EXIT_HORIZON);
    }

    #[test]
    fn a_piece_boxed_in_at_home_near_the_deadline_is_a_danger() {
        let evaluator = Evaluator::new(HomeSide::Left);
        let danger = |fen: &str, ply| {
            let board = board(state(fen, ply));
            evaluator.rush(Color::White, board.white, &board)
        };
        let free = danger("W:Wd3,h8:Ba5,b5,a6,b6", 72);
        let boxed = danger("W:Wd3,h8:Bd4,d5,e3,f3", 72);
        assert!(0 < free && free < boxed);
        // Later the same box leaves too few moves to get out.
        assert!(danger("W:Wd3,h8:Bd4,d5,e3,f3", 78) > boxed + evaluator.weights.crisis);
        assert_eq!(danger("W:Wd3,h8:Bd4,d5,e3,f3", 20), 0);
        assert_eq!(danger("W:We4,h8:Bd4,d5,e3,f3", 72), 0);
    }

    #[test]
    fn sparing_a_side_drops_only_its_deadline_danger() {
        // Black to move; White's piece d3 is boxed in at home with few moves left.
        let board = board(state("B:Wd3,h8:Bd4,d5,e3,f3,a1", 77));
        let full = Evaluator::new(HomeSide::Left);
        let danger = full.rush(Color::White, board.white, &board);
        assert!(danger > full.weights.crisis);
        let spared = Evaluator::new(HomeSide::Left).sparing(Some(Color::White));
        assert_eq!(spared.rush(Color::White, board.white, &board), 0);
        assert_eq!(full.evaluate(&board) - spared.evaluate(&board), danger);
        // Black has no piece at home here: sparing Black changes nothing.
        let black_spared = Evaluator::new(HomeSide::Left).sparing(Some(Color::Black));
        assert_eq!(black_spared.evaluate(&board), full.evaluate(&board));
        // The value stays the opposite for the two sides to move.
        let mut white_to_move = board;
        white_to_move.side = Color::White;
        white_to_move.ply += 1;
        assert_eq!(
            full.evaluate(&white_to_move) - spared.evaluate(&white_to_move),
            -full.rush(Color::White, white_to_move.white, &white_to_move)
        );
    }

    #[test]
    fn pieces_that_stay_at_home_late_are_penalised() {
        let early = state("W:Wa1:Bd4", 20);
        let late = state("W:Wa1:Bd4", 76);
        assert!(evaluate(late, Color::White) < evaluate(early, Color::White));
    }

    #[test]
    fn both_layouts_are_mirror_images() {
        let left = Evaluator::new(HomeSide::Left);
        let right = Evaluator::new(HomeSide::Right);
        let mirror = |mask: u64| {
            squares_of(mask).fold(0, |m, sq| {
                m | Square::from_file_rank(7 - sq.file(), sq.rank()).unwrap().bit()
            })
        };
        let mut rng = Rng::new(0x3131);
        for _ in 0..200 {
            let mut cells: Vec<_> = Square::all().collect();
            rng.shuffle(&mut cells);
            let pieces = cells[..rng.below(PIECES + 1)]
                .iter()
                .fold(0, |mask, sq| mask | sq.bit());
            for color in [Color::White, Color::Black] {
                assert_eq!(
                    left.profile(color, pieces).work,
                    right.profile(color, mirror(pieces)).work
                );
            }
        }
    }
}
