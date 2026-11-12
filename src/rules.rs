//! Окончание партии: выигрыш и автоматические ничьи.
//!
//! «N ходов» в правилах означает N ходов каждого игрока, поэтому счётчики, которые
//! считают ходы обеих сторон, ведутся в полуходах и сравниваются с удвоенным лимитом.
//! Счётчики, считающие ходы одной (сильнейшей) стороны, ведутся в её ходах.
//!
//! «Соотношение сил» — число простых и дамок каждой стороны. Оно меняется при любом
//! взятии и при любом превращении простой в дамку; при этом счётчики правил 2, 4 и 5
//! сбрасываются.

use crate::board::{Color, Position, MAIN_ROAD};
use crate::moves::{generate_moves, Move};

/// Правило 1: позиция с той же очередью хода возникла в третий раз.
pub const REPETITION_LIMIT: u32 = 3;
/// Правило 2: три и более дамки против одной дамки — столько ходов сильнейшей стороны
/// отводится на взятие с момента возникновения соотношения.
pub const KINGS_VS_KING_MOVES: u32 = 15;
/// Правило 3: столько ходов подряд обе стороны ходят только дамками без взятий.
pub const KINGS_ONLY_MOVES: u32 = 15;
/// Правило 4: лимит ходов без изменения соотношения сил при 2–3 фигурах на доске.
pub const BALANCE_MOVES_2_3: u32 = 5;
/// Правило 4: лимит ходов при 4–5 фигурах.
pub const BALANCE_MOVES_4_5: u32 = 30;
/// Правило 4: лимит ходов при 6–7 фигурах.
pub const BALANCE_MOVES_6_7: u32 = 60;
/// Правило 5: три шашки против одинокой дамки на большой дороге — столько ходов
/// сильнейшей стороны отводится на взятие.
pub const MAIN_ROAD_MOVES: u32 = 5;

/// Причина ничьей.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum DrawReason {
    /// Правило 1: троекратное повторение позиции.
    Repetition,
    /// Правило 2: три и более дамки не взяли одинокую дамку за 15 ходов.
    KingsVsKing,
    /// Правило 3: 15 ходов только дамками без взятий.
    KingsOnly,
    /// Правило 4: соотношение сил не менялось заданное число ходов.
    /// `pieces` — число фигур на доске, `moves` — сработавший лимит.
    Balance { pieces: u32, moves: u32 },
    /// Правило 5: три шашки не взяли одинокую дамку на большой дороге за 5 ходов.
    MainRoad,
}

/// Причина выигрыша.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum WinReason {
    /// У проигравшего не осталось шашек.
    NoPieces,
    /// У проигравшего нет допустимых ходов.
    NoMoves,
}

/// Результат завершённой партии.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Outcome {
    Win { winner: Color, reason: WinReason },
    Draw(DrawReason),
}

impl Outcome {
    /// Победитель, если партия не закончилась вничью.
    #[must_use]
    pub const fn winner(&self) -> Option<Color> {
        match self {
            Self::Win { winner, .. } => Some(*winner),
            Self::Draw(_) => None,
        }
    }

    #[must_use]
    pub const fn is_draw(&self) -> bool {
        matches!(self, Self::Draw(_))
    }
}

/// Счётчики правил ничьей, зависящие от хода партии.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct DrawCounters {
    /// Правило 2: ходы сильнейшей стороны с момента возникновения соотношения
    /// «три и более дамки против одной дамки».
    pub kings_vs_king: u8,
    /// Правило 3: полуходы подряд, сделанные дамками без взятий.
    pub kings_only: u8,
    /// Правило 4: полуходы без изменения соотношения сил, пока у обеих сторон есть
    /// дамки и на доске не более семи фигур.
    pub balance: u8,
    /// Правило 5: ходы сильнейшей стороны, пока три её шашки стоят против одинокой
    /// дамки на большой дороге.
    pub main_road: u8,
}

/// Правило 2: сторона, у которой три и более дамки против одинокой дамки соперника.
#[must_use]
pub fn kings_vs_king_side(pos: &Position) -> Option<Color> {
    for strong in [Color::White, Color::Black] {
        let weak = strong.opponent();
        // The weaker side has a single piece, and that piece is a king.
        if pos.kings_of(strong).count_ones() >= 3 && pos.count(weak) == 1 && pos.kings_of(weak) != 0 {
            return Some(strong);
        }
    }
    None
}

/// Правило 5: сторона, у которой ровно три шашки (в любом сочетании дамок и простых)
/// против одинокой дамки соперника, стоящей на большой дороге.
#[must_use]
pub fn main_road_side(pos: &Position) -> Option<Color> {
    for strong in [Color::White, Color::Black] {
        let weak = strong.opponent();
        if pos.count(strong) == 3 && pos.count(weak) == 1 && pos.kings_of(weak) & MAIN_ROAD != 0 {
            return Some(strong);
        }
    }
    None
}

/// Правило 4: лимит ходов для текущего числа фигур, если у обеих сторон есть дамки.
#[must_use]
pub const fn balance_limit(pos: &Position) -> Option<u32> {
    if pos.kings_of(Color::White) == 0 || pos.kings_of(Color::Black) == 0 {
        return None;
    }
    match pos.total() {
        2 | 3 => Some(BALANCE_MOVES_2_3),
        4 | 5 => Some(BALANCE_MOVES_4_5),
        6 | 7 => Some(BALANCE_MOVES_6_7),
        _ => None,
    }
}

/// Состояние партии: позиция и счётчики ничьей, накопленные по ветке, ведущей к ней.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct GameState {
    pub pos: Position,
    pub counters: DrawCounters,
}

impl GameState {
    /// Состояние с обнулёнными счётчиками.
    #[must_use]
    pub fn new(pos: Position) -> Self {
        Self {
            pos,
            counters: DrawCounters::default(),
        }
    }

    /// Начало партии.
    #[must_use]
    pub fn initial() -> Self {
        Self::new(Position::initial())
    }

    /// Состояние после хода `mv` (ход должен быть допустимым в этой позиции).
    #[must_use]
    pub fn play(&self, mv: &Move) -> Self {
        let mover = self.pos.side;
        let king_move = self.pos.kings & mv.from().bit() != 0;
        let capture = mv.is_capture();
        let balance_changed = capture || mv.promotes();
        let next = self.pos.after(mv);
        let old = self.counters;

        // Правило 3: считаются только тихие ходы дамок.
        let kings_only = if king_move && !capture {
            old.kings_only.saturating_add(1)
        } else {
            0
        };

        // Правило 4.
        let balance = if !balance_changed && balance_limit(&next).is_some() {
            old.balance.saturating_add(1)
        } else {
            0
        };

        // Правило 2: считаются ходы сильнейшей стороны.
        let kings_vs_king = match kings_vs_king_side(&next) {
            Some(strong) if !balance_changed => {
                if mover == strong {
                    old.kings_vs_king.saturating_add(1)
                } else {
                    old.kings_vs_king
                }
            }
            _ => 0,
        };

        // Правило 5: отсчёт начинается, когда одинокая дамка оказалась на большой дороге
        // при трёх шашках соперника, и прерывается, если она дорогу покинула.
        let main_road = match main_road_side(&next) {
            Some(strong) if !balance_changed && main_road_side(&self.pos) == Some(strong) => {
                if mover == strong {
                    old.main_road.saturating_add(1)
                } else {
                    old.main_road
                }
            }
            _ => 0,
        };

        Self {
            pos: next,
            counters: DrawCounters {
                kings_vs_king,
                kings_only,
                balance,
                main_road,
            },
        }
    }

    /// Ничья по счётчикам (правила 2–5), если один из лимитов достигнут.
    #[must_use]
    pub fn counter_draw(&self) -> Option<DrawReason> {
        let c = &self.counters;
        if u32::from(c.kings_vs_king) >= KINGS_VS_KING_MOVES {
            return Some(DrawReason::KingsVsKing);
        }
        if u32::from(c.kings_only) >= 2 * KINGS_ONLY_MOVES {
            return Some(DrawReason::KingsOnly);
        }
        if let Some(limit) = balance_limit(&self.pos) {
            if u32::from(c.balance) >= 2 * limit {
                return Some(DrawReason::Balance {
                    pieces: self.pos.total(),
                    moves: limit,
                });
            }
        }
        if u32::from(c.main_road) >= MAIN_ROAD_MOVES {
            return Some(DrawReason::MainRoad);
        }
        None
    }

    /// Через сколько полуходов самое раннее из правил 2–5 зафиксирует ничью, если
    /// соотношение сил не изменится и простые не будут ходить.
    /// `u32::MAX`, если ни одно правило сейчас не отсчитывается.
    #[must_use]
    pub fn plies_to_counter_draw(&self) -> u32 {
        let c = &self.counters;
        let mut best = u32::MAX;
        if self.pos.kings != 0 {
            best = best.min((2 * KINGS_ONLY_MOVES).saturating_sub(u32::from(c.kings_only)));
        }
        if let Some(limit) = balance_limit(&self.pos) {
            best = best.min((2 * limit).saturating_sub(u32::from(c.balance)));
        }
        let one_sided = |limit: u32, done: u8, strong: Color| -> u32 {
            let left = limit.saturating_sub(u32::from(done));
            if left == 0 {
                0
            } else if self.pos.side == strong {
                2 * left - 1
            } else {
                2 * left
            }
        };
        if let Some(strong) = kings_vs_king_side(&self.pos) {
            best = best.min(one_sided(KINGS_VS_KING_MOVES, c.kings_vs_king, strong));
        }
        if let Some(strong) = main_road_side(&self.pos) {
            best = best.min(one_sided(MAIN_ROAD_MOVES, c.main_road, strong));
        }
        best
    }

    /// Результат партии в этом состоянии.
    ///
    /// `occurrences` — сколько раз эта позиция (с той же очередью хода) встретилась
    /// на ветке, ведущей к ней, считая текущую.
    ///
    /// Сначала проверяется проигрыш стороны, которой нечем ходить: ход, лишивший
    /// соперника ходов, выигрывает, даже если тем же ходом исчерпан лимит ничьей.
    pub fn outcome(&self, occurrences: u32) -> Option<Outcome> {
        let mut moves = Vec::new();
        generate_moves(&self.pos, &mut moves);
        if moves.is_empty() {
            let loser = self.pos.side;
            let reason = if self.pos.count(loser) == 0 {
                WinReason::NoPieces
            } else {
                WinReason::NoMoves
            };
            return Some(Outcome::Win {
                winner: loser.opponent(),
                reason,
            });
        }
        if occurrences >= REPETITION_LIMIT {
            return Some(Outcome::Draw(DrawReason::Repetition));
        }
        self.counter_draw().map(Outcome::Draw)
    }
}

/// Сколько раз позиция `current` встречается среди `earlier` (более ранних позиций
/// ветки), считая её саму.
#[must_use]
pub fn occurrences(earlier: &[Position], current: &Position) -> u32 {
    1 + earlier.iter().filter(|p| *p == current).count() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_match_the_rules() {
        assert_eq!(REPETITION_LIMIT, 3);
        assert_eq!(KINGS_VS_KING_MOVES, 15);
        assert_eq!(KINGS_ONLY_MOVES, 15);
        assert_eq!(
            (BALANCE_MOVES_2_3, BALANCE_MOVES_4_5, BALANCE_MOVES_6_7),
            (5, 30, 60)
        );
        assert_eq!(MAIN_ROAD_MOVES, 5);
    }

    #[test]
    fn balance_limit_depends_on_piece_count() {
        let p = |fen: &str| Position::from_fen(fen).unwrap();
        assert_eq!(balance_limit(&p("W:WKa1:BKh8")), Some(5));
        assert_eq!(balance_limit(&p("W:WKa1,c1:BKh8")), Some(5));
        assert_eq!(balance_limit(&p("W:WKa1,c1,e1:BKh8")), Some(30));
        assert_eq!(balance_limit(&p("W:WKa1,c1,e1:BKh8,b8")), Some(30));
        assert_eq!(balance_limit(&p("W:WKa1,c1,e1:BKh8,b8,d8")), Some(60));
        assert_eq!(balance_limit(&p("W:WKa1,c1,e1,g1:BKh8,b8,d8")), Some(60));
        assert_eq!(balance_limit(&p("W:WKa1,c1,e1,g1:BKh8,b8,d8,f8")), None);
        assert_eq!(balance_limit(&p("W:WKa1,c1:Bh8")), None, "у чёрных нет дамки");
        assert_eq!(balance_limit(&Position::initial()), None);
    }

    #[test]
    fn strong_side_detection() {
        let p = |fen: &str| Position::from_fen(fen).unwrap();
        assert_eq!(kings_vs_king_side(&p("W:WKa1,Kc1,Ke1:BKh8")), Some(Color::White));
        assert_eq!(
            kings_vs_king_side(&p("W:WKa1:BKh8,Kb8,Kd8,Kf8")),
            Some(Color::Black)
        );
        assert_eq!(
            kings_vs_king_side(&p("W:WKa1,Kc1,e1:BKh8")),
            None,
            "только две дамки"
        );
        assert_eq!(
            kings_vs_king_side(&p("W:WKa1,Kc1,Ke1:Bh8")),
            None,
            "простая, а не дамка"
        );
        assert_eq!(
            kings_vs_king_side(&p("W:WKa1,Kc1,Ke1:BKh8,b8")),
            None,
            "дамка не одна"
        );
        assert_eq!(
            kings_vs_king_side(&p("W:WKa1,Kc1,Ke1,g1:BKh8")),
            Some(Color::White)
        );

        assert_eq!(main_road_side(&p("W:Wc1,e1,g1:BKh8")), Some(Color::White));
        assert_eq!(main_road_side(&p("W:WKc1,e1,g1:BKd4")), Some(Color::White));
        assert_eq!(
            main_road_side(&p("W:WKc1,e1,g1:BKb8")),
            None,
            "дамка не на большой дороге"
        );
        assert_eq!(main_road_side(&p("W:WKc1,e1,g1:Bd4")), None, "на дороге простая");
        assert_eq!(main_road_side(&p("W:WKc1,e1:BKd4")), None, "две шашки");
        assert_eq!(main_road_side(&p("W:WKc1,e1,g1,a3:BKd4")), None, "четыре шашки");
        assert_eq!(main_road_side(&p("W:WKd4:BKc1,e3,g3")), Some(Color::Black));
    }
}
