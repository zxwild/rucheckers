//! Race evaluation: distinct destinations, lagging pieces and available jump chains.

use super::{home_for, State, HOME_DEADLINE};
use crate::games::{squares_of, Color, Position, Square};

fn distance(a: Square, b: Square) -> i32 {
    i32::from(a.file().abs_diff(b.file()) + a.rank().abs_diff(b.rank()))
}

/// All destinations of a jump chain, without applying turn-ending restrictions.
/// This is a positional feature, not the legal move generator.
fn jumps(origin: Square, occupied: u64) -> u64 {
    const A_FILE: u64 = 0x0101_0101_0101_0101;
    const H_FILE: u64 = 0x8080_8080_8080_8080;
    let occupied = occupied & !origin.bit();
    let mut reached = origin.bit();
    loop {
        let next = ((((reached << 8) & occupied) << 8)
            | (((reached >> 8) & occupied) >> 8)
            | (((((reached & !H_FILE) << 1) & occupied) & !H_FILE) << 1)
            | (((((reached & !A_FILE) >> 1) & occupied) & !A_FILE) >> 1))
            & !occupied;
        let extended = reached | next;
        if extended == reached {
            return reached & !origin.bit();
        }
        reached = extended;
    }
}

fn travel_cost(distance: i32) -> i32 {
    // Quadratic cost makes a stranded rear piece more urgent than a front runner.
    distance * (8 + distance)
}

/// Match every piece to a different destination. A ready chain reduces the estimated
/// travel cost, so preparing a stepping stone is useful before the jump is played.
fn assignment(pos: Position, color: Color) -> i32 {
    let pieces: Vec<_> = squares_of(pos.pieces(color)).collect();
    let goals: Vec<_> = squares_of(home_for(color.opponent(), pos.home_side)).collect();
    let mut costs = [[0; 12]; 12];
    for (i, &piece) in pieces.iter().enumerate() {
        let reachable = jumps(piece, pos.occupied());
        for (j, &goal) in goals.iter().enumerate() {
            let direct = travel_cost(distance(piece, goal));
            let ready = squares_of(reachable)
                .map(|landing| 12 + travel_cost(distance(landing, goal)))
                .min()
                .unwrap_or(direct)
                .min(direct);
            costs[i][j] = direct - (direct - ready) / 2;
        }
    }
    // Rectangular Hungarian assignment, also defined for reduced test positions.
    let mut u = [0i32; 13];
    let mut v = [0i32; 13];
    let mut matched = [0usize; 13];
    let mut previous = [0usize; 13];
    for row in 1..=pieces.len() {
        matched[0] = row;
        let mut column = 0;
        let mut minima = [i32::MAX; 13];
        let mut used = [false; 13];
        loop {
            used[column] = true;
            let current = matched[column];
            let mut delta = i32::MAX;
            let mut next = 0;
            for j in 1..=goals.len() {
                if used[j] {
                    continue;
                }
                let cost = costs[current - 1][j - 1] - u[current] - v[j];
                if cost < minima[j] {
                    minima[j] = cost;
                    previous[j] = column;
                }
                if minima[j] < delta {
                    delta = minima[j];
                    next = j;
                }
            }
            for j in 0..=goals.len() {
                if used[j] {
                    u[matched[j]] += delta;
                    v[j] -= delta;
                } else {
                    minima[j] = minima[j].saturating_sub(delta);
                }
            }
            column = next;
            if matched[column] == 0 {
                break;
            }
        }
        loop {
            let next = previous[column];
            matched[column] = matched[next];
            column = next;
            if column == 0 {
                break;
            }
        }
    }
    -v[0]
}

#[must_use]
pub fn evaluate(state: State, color: Color) -> i32 {
    let score = |side: Color| {
        let target = home_for(side.opponent(), state.pos.home_side);
        let arrived = (state.pos.pieces(side) & target).count_ones() as i32;
        let own_home = home_for(side, state.pos.home_side);
        let elapsed = i32::from(state.ply.min(HOME_DEADLINE * 2));
        let urgency = 20 + elapsed * 3;
        // Leaving a stranded back-row piece can take several turns. Reward each
        // preparatory step towards the exit, not only the final step over the edge.
        let exit_cost: i32 = squares_of(state.pos.pieces(side) & own_home)
            .map(|piece| {
                let quiet_exit = |square: Square| {
                    if own_home & square.bit() == 0 {
                        return 0;
                    }
                    let file = if own_home & 1 != 0 || own_home & (1u64 << 56) != 0 {
                        square.file()
                    } else {
                        7 - square.file()
                    };
                    let rank = if side == Color::White {
                        square.rank()
                    } else {
                        7 - square.rank()
                    };
                    i32::from((4 - file).min(3 - rank))
                };
                let turns = squares_of(jumps(piece, state.pos.occupied()))
                    .map(|landing| 1 + quiet_exit(landing))
                    .min()
                    .unwrap_or(4)
                    .min(quiet_exit(piece));
                urgency * (turns + 1) / 2
            })
            .sum();
        // Fill the back of the destination before crowding its entrance.
        let corner_file = if target & 1 != 0 || target & (1u64 << 56) != 0 {
            0
        } else {
            7
        };
        let corner_rank = if side == Color::White { 7 } else { 0 };
        let depth: i32 = squares_of(state.pos.pieces(side) & target)
            .map(|sq| 5 - i32::from(sq.file().abs_diff(corner_file) + sq.rank().abs_diff(corner_rank)))
            .sum();
        arrived * 70 + depth * 12 - assignment(state.pos, side) * 2 - exit_cost
    };
    score(color) - score(color.opponent())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::{GameKind, Kind, Move, Piece};
    use crate::rng::Rng;

    fn sq(name: &str) -> Square {
        Square::parse(name).unwrap()
    }

    #[test]
    fn jump_potential_matches_coordinate_flood_fill_on_every_cell() {
        let mut rng = Rng::new(0xBF31);
        for _ in 0..100 {
            let occupied = rng.next_u64();
            for origin in Square::all() {
                let pieces = occupied & !origin.bit();
                let mut visited = vec![origin];
                let mut index = 0;
                while index < visited.len() {
                    let from = visited[index];
                    index += 1;
                    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                        let x = i32::from(from.file()) + 2 * dx;
                        let y = i32::from(from.rank()) + 2 * dy;
                        if !(0..8).contains(&x) || !(0..8).contains(&y) {
                            continue;
                        }
                        let over = Square::from_file_rank((x - dx) as u8, (y - dy) as u8).unwrap();
                        let to = Square::from_file_rank(x as u8, y as u8).unwrap();
                        if pieces & over.bit() != 0 && pieces & to.bit() == 0 && !visited.contains(&to) {
                            visited.push(to);
                        }
                    }
                }
                let expected = visited.iter().fold(0, |mask, sq| mask | sq.bit()) & !origin.bit();
                assert_eq!(jumps(origin, occupied), expected, "{origin}, {occupied:016x}");
            }
        }
    }

    #[test]
    fn preparing_a_jump_reduces_the_race_cost_before_the_piece_moves() {
        let mut waiting = Position::from_fen_for(GameKind::Ugolki, "W:Wa1:Bh2,h4,h6").unwrap();
        let before = assignment(waiting, Color::White);
        waiting.black = sq("a2").bit() | sq("a4").bit() | sq("a6").bit();
        assert!(assignment(waiting, Color::White) < before);
        assert_eq!(waiting.white, sq("a1").bit());
    }

    #[test]
    fn a_stranded_rear_piece_matters_more_than_decorating_a_finished_corner() {
        let state = State {
            pos: Position::from_fen_for(GameKind::Ugolki, "W:Wa1,h7:Be4").unwrap(),
            ply: 60,
        };
        let rescue = state.after(&Move::route(sq("a1"), &[sq("a2")]));
        let front = state.after(&Move::route(sq("h7"), &[sq("h8")]));
        assert!(evaluate(rescue, Color::White) > evaluate(front, Color::White));
    }

    #[test]
    fn the_back_of_the_goal_is_filled_before_its_entrance() {
        let mut near = Position::empty(Color::White, GameKind::Ugolki);
        near.put(sq("e6"), Piece::new(Color::White, Kind::Man));
        let mut deep = near;
        deep.remove(sq("e6"));
        deep.put(sq("h8"), Piece::new(Color::White, Kind::Man));
        assert!(evaluate(State::new(deep), Color::White) > evaluate(State::new(near), Color::White));
    }
}
