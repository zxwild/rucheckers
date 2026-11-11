//! Поиск лучшего хода: итеративное углубление, альфа-бета с главным вариантом,
//! таблица перестановок, упорядочение ходов и сокращение поздних ходов.
//!
//! Особенности шашек, учтённые в поиске:
//! * взятия обязательны, поэтому они не расходуют глубину, а статическая оценка
//!   берётся только в позициях, где стороне нечего бить;
//! * правила ничьей (повторение и счётчики) проверяются в каждом узле;
//! * оценка плавно убывает к нулю при приближении ничьей по счётчику, чтобы
//!   сильнейшая сторона вовремя меняла соотношение сил.

use super::eval::evaluate;
use crate::board::Position;
use crate::moves::{generate_moves, legal_moves, Move};
use crate::rng::Rng;
use crate::rules::GameState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Оценка выигрыша в текущем узле; выигрыш через `n` полуходов оценивается как `WIN - n`.
pub const WIN: i32 = 30_000;
/// Граница, выше которой оценка означает форсированный выигрыш.
pub const WIN_BOUND: i32 = WIN - 1_000;
const INF: i32 = 32_000;
/// Наибольшая глубина линии поиска в полуходах.
const MAX_PLY: usize = 120;
/// За столько полуходов до ничьей по счётчику оценка начинает убывать к нулю.
const DRAW_FADE_PLIES: u32 = 16;
/// Если до ничьей по счётчику осталось меньше полуходов, остаток входит в ключ таблицы.
const DRAW_KEY_PLIES: u32 = 24;

const BOUND_NONE: u8 = 0;
const BOUND_EXACT: u8 = 1;
const BOUND_LOWER: u8 = 2;
const BOUND_UPPER: u8 = 3;
const NO_MOVE: u8 = u8::MAX;
const NO_KILLER: u16 = u16::MAX;
/// Объём работы (узлы и сгенерированные ходы) между проверками времени и сигнала остановки.
const POLL_WORK: usize = 2_048;

/// Параметры одного поиска.
#[derive(Clone, Debug)]
pub struct SearchParams {
    /// Наибольшая глубина в тихих полуходах (взятия глубину не расходуют).
    pub max_depth: i32,
    /// Жёсткий лимит времени: по его истечении поиск прерывается.
    pub hard_time: Option<Duration>,
    /// Мягкий лимит: после него новая итерация не начинается.
    pub soft_time: Option<Duration>,
    /// Доводить ли взятия до конца на границе глубины.
    pub quiescence: bool,
    /// Амплитуда псевдослучайной добавки к оценке (в сотых простой).
    pub jitter: i32,
    /// Зерно: порядок равноценных ходов и добавка к оценке.
    pub seed: u64,
    /// Размер таблицы перестановок: 2^`table_bits` записей.
    pub table_bits: u32,
    /// Считать точную оценку каждого хода в корне (нужно уровням со случайным выбором).
    pub exact_root_scores: bool,
}

/// Результат поиска.
#[derive(Clone, Debug)]
pub struct SearchResult {
    /// Лучший ход; `None`, если ходов нет или поиск прерван до первой оценки.
    pub best: Option<Move>,
    /// Оценка лучшего хода с точки зрения стороны, чья очередь хода.
    pub score: i32,
    /// Глубина последней полностью завершённой итерации.
    pub depth: i32,
    /// Число просмотренных узлов.
    pub nodes: u64,
    /// Оценки ходов в корне по последней завершённой итерации.
    /// Точны для всех ходов только при `exact_root_scores`.
    pub root_scores: Vec<(Move, i32)>,
    /// Поиск остановлен по времени или по внешнему сигналу.
    pub aborted: bool,
}

#[derive(Clone, Copy)]
struct Entry {
    key: u64,
    score: i16,
    depth: i8,
    bound: u8,
    mv: u8,
}

const EMPTY_ENTRY: Entry = Entry {
    key: 0,
    score: 0,
    depth: 0,
    bound: BOUND_NONE,
    mv: NO_MOVE,
};

/// Таблица перестановок: корзины по две записи (предпочтение глубине и всегда заменяемая).
struct Table {
    entries: Vec<Entry>,
    mask: usize,
}

impl Table {
    fn new(bits: u32) -> Table {
        let size = 1usize << bits.clamp(4, 26);
        Table {
            entries: vec![EMPTY_ENTRY; size],
            mask: size - 2,
        }
    }

    #[inline]
    fn probe(&self, key: u64) -> Option<Entry> {
        let index = (key as usize) & self.mask;
        let first = self.entries[index];
        if first.bound != BOUND_NONE && first.key == key {
            return Some(first);
        }
        let second = self.entries[index + 1];
        if second.bound != BOUND_NONE && second.key == key {
            return Some(second);
        }
        None
    }

    #[inline]
    fn store(&mut self, key: u64, depth: i32, score: i32, bound: u8, mv: u8) {
        let index = (key as usize) & self.mask;
        let entry = Entry {
            key,
            score: score.clamp(-INF, INF) as i16,
            depth: depth.clamp(0, 127) as i8,
            bound,
            mv,
        };
        let first = self.entries[index];
        if first.bound == BOUND_NONE || first.key == key || entry.depth >= first.depth {
            self.entries[index] = entry;
        } else {
            self.entries[index + 1] = entry;
        }
    }
}

#[inline]
fn mix(mut x: u64) -> u64 {
    x ^= x >> 33;
    x = x.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    x ^= x >> 33;
    x
}

#[inline]
fn to_table(score: i32, ply: usize) -> i32 {
    if score >= WIN_BOUND {
        score + ply as i32
    } else if score <= -WIN_BOUND {
        score - ply as i32
    } else {
        score
    }
}

#[inline]
fn from_table(score: i32, ply: usize) -> i32 {
    if score >= WIN_BOUND {
        score - ply as i32
    } else if score <= -WIN_BOUND {
        score + ply as i32
    } else {
        score
    }
}

/// Убирает взятия, которые отличаются только маршрутом: если совпадают исходное и
/// конечное поля и набор побитых шашек, позиция после хода одна и та же, и для поиска
/// такие ходы равноценны.
fn dedupe_captures(moves: &mut Vec<Move>) {
    if moves.len() < 2 || !moves[0].is_capture() {
        return;
    }
    let mut kept = 1;
    for i in 1..moves.len() {
        let mv = moves[i];
        let duplicate = moves[..kept].iter().any(|k| {
            k.to() == mv.to()
                && k.captured() == mv.captured()
                && k.from() == mv.from()
                && k.promotes() == mv.promotes()
        });
        if !duplicate {
            moves[kept] = mv;
            kept += 1;
        }
    }
    moves.truncate(kept);
}

#[inline]
fn killer_key(mv: &Move) -> u16 {
    (mv.from().index() as u16) << 5 | mv.to().index() as u16
}

struct Searcher<'a> {
    table: Table,
    stop: &'a AtomicBool,
    started: Instant,
    hard_time: Option<Duration>,
    nodes: u64,
    /// Объём работы с последней проверки времени (узлы и сгенерированные ходы).
    work: usize,
    aborted: bool,
    /// Позиции ветки партии до корня и текущей линии поиска.
    path: Vec<Position>,
    /// Индекс корневой позиции в `path`.
    root_index: usize,
    killers: Vec<[u16; 2]>,
    history: Vec<i32>,
    move_buffers: Vec<Vec<Move>>,
    order_buffers: Vec<Vec<(i32, u16)>>,
    quiescence: bool,
    jitter: i32,
    seed: u64,
}

impl Searcher<'_> {
    fn poll(&mut self) {
        if self.stop.load(Ordering::Relaxed) {
            self.aborted = true;
        } else if let Some(limit) = self.hard_time {
            if self.started.elapsed() >= limit {
                self.aborted = true;
            }
        }
    }

    /// Статическая оценка с точки зрения стороны, чья очередь хода.
    fn eval(&self, st: &GameState) -> i32 {
        let mut score = evaluate(&st.pos);
        if self.jitter > 0 {
            let span = 2 * self.jitter as u64 + 1;
            score += (mix(st.pos.hash64() ^ self.seed) % span) as i32 - self.jitter;
        }
        let left = st.plies_to_counter_draw();
        if left < DRAW_FADE_PLIES {
            score = score * left as i32 / DRAW_FADE_PLIES as i32;
        }
        score.clamp(-WIN_BOUND + 1, WIN_BOUND - 1)
    }

    /// Ключ таблицы: позиция и, вблизи ничьей по счётчику, оставшееся до неё число полуходов.
    fn key(&self, st: &GameState) -> u64 {
        let left = st.plies_to_counter_draw();
        let salt = if left < DRAW_KEY_PLIES {
            mix(0xD6E8_FEB8_6659_FD93 ^ u64::from(left))
        } else {
            0
        };
        st.pos.hash64() ^ salt
    }

    /// Повторение позиции: внутри линии поиска достаточно одного совпадения,
    /// среди позиций партии до корня нужно два (тогда это третье появление).
    fn repeated(&self, st: &GameState) -> bool {
        let reversible = st.counters.kings_only as usize;
        if reversible < 4 {
            return false;
        }
        let len = self.path.len();
        let mut earlier = 0;
        let mut back = 4;
        while back <= reversible && back <= len {
            let index = len - back;
            if self.path[index] == st.pos {
                if index >= self.root_index {
                    return true;
                }
                earlier += 1;
                if earlier >= 2 {
                    return true;
                }
            }
            back += 2;
        }
        false
    }

    fn negamax(&mut self, st: &GameState, depth: i32, alpha: i32, beta: i32, ply: usize, pv: bool) -> i32 {
        self.nodes += 1;
        if self.aborted {
            return 0;
        }
        let mut moves = std::mem::take(&mut self.move_buffers[ply]);
        let mut order = std::mem::take(&mut self.order_buffers[ply]);
        generate_moves(&st.pos, &mut moves);
        // Время проверяется по объёму работы, а не по числу узлов: в позициях с длинными
        // взятиями дамок один узел может содержать сотни ходов.
        self.work += 1 + moves.len();
        if self.work >= POLL_WORK {
            self.work = 0;
            self.poll();
        }
        dedupe_captures(&mut moves);
        let score = if self.aborted {
            0
        } else {
            self.node(st, depth, alpha, beta, ply, pv, &moves, &mut order)
        };
        self.move_buffers[ply] = moves;
        self.order_buffers[ply] = order;
        score
    }

    #[allow(clippy::too_many_arguments)]
    fn node(
        &mut self,
        st: &GameState,
        depth: i32,
        mut alpha: i32,
        mut beta: i32,
        ply: usize,
        pv: bool,
        moves: &[Move],
        order: &mut Vec<(i32, u16)>,
    ) -> i32 {
        if moves.is_empty() {
            return -(WIN - ply as i32);
        }
        if st.counter_draw().is_some() || self.repeated(st) {
            return 0;
        }
        let in_capture = moves[0].is_capture();
        if ply + 1 >= MAX_PLY || (depth <= 0 && !(in_capture && self.quiescence)) {
            return self.eval(st);
        }
        let depth = depth.max(0);

        // Быстрее, чем за `ply` полуходов, уже не выиграть и не проиграть.
        alpha = alpha.max(-(WIN - ply as i32));
        beta = beta.min(WIN - ply as i32 - 1);
        if alpha >= beta {
            return alpha;
        }

        let key = self.key(st);
        let mut table_move = NO_MOVE;
        if let Some(entry) = self.table.probe(key) {
            table_move = entry.mv;
            if !pv && i32::from(entry.depth) >= depth {
                let score = from_table(i32::from(entry.score), ply);
                match entry.bound {
                    BOUND_EXACT => return score,
                    BOUND_LOWER if score >= beta => return score,
                    BOUND_UPPER if score <= alpha => return score,
                    _ => {}
                }
            }
        }

        // Упорядочение: ход из таблицы, затем крупные взятия, «убийцы» и история.
        let side = st.pos.side as usize;
        let killers = self.killers[ply];
        order.clear();
        for (index, mv) in moves.iter().enumerate() {
            let score = if index == table_move as usize {
                1_000_000_000
            } else if in_capture {
                let kings = (mv.captured() & st.pos.kings).count_ones() as i32;
                1_000 * mv.captured_count() as i32 + 500 * kings + 100 * i32::from(mv.promotes())
            } else {
                let k = killer_key(mv);
                if k == killers[0] {
                    900_000_000
                } else if k == killers[1] {
                    800_000_000
                } else {
                    let h = self.history[side * 1024 + k as usize];
                    h + if mv.promotes() { 500_000_000 } else { 0 }
                }
            };
            order.push((score, index as u16));
        }

        let count = moves.len();
        // Короткие списки упорядочиваются выбором по ходу перебора (часто хватает первого
        // хода), длинные сортируются сразу.
        let presorted = count > 24;
        if presorted {
            order.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        }
        let step = i32::from(!(in_capture || count == 1));
        let original_alpha = alpha;
        let mut best = -INF;
        let mut best_index = NO_MOVE;
        self.path.push(st.pos);
        for i in 0..count {
            if !presorted {
                // Выбор лучшего из оставшихся ходов.
                let mut pick = i;
                for j in i + 1..count {
                    if order[j].0 > order[pick].0 {
                        pick = j;
                    }
                }
                order.swap(i, pick);
            }
            let index = order[i].1 as usize;
            let mv = &moves[index];
            let child = st.play(mv);
            let next_depth = depth - step;

            let mut score;
            if i == 0 {
                score = -self.negamax(&child, next_depth, -beta, -alpha, ply + 1, pv);
            } else {
                let mut reduction = 0;
                if !in_capture && depth >= 3 && i >= 3 && !mv.promotes() {
                    reduction = if depth >= 6 && i >= 8 { 2 } else { 1 };
                }
                score = -self.negamax(&child, next_depth - reduction, -alpha - 1, -alpha, ply + 1, false);
                if score > alpha && reduction > 0 {
                    score = -self.negamax(&child, next_depth, -alpha - 1, -alpha, ply + 1, false);
                }
                if score > alpha && score < beta {
                    score = -self.negamax(&child, next_depth, -beta, -alpha, ply + 1, pv);
                }
            }
            if self.aborted {
                break;
            }
            if score > best {
                best = score;
                best_index = index.min(NO_MOVE as usize) as u8;
            }
            if score > alpha {
                alpha = score;
            }
            if alpha >= beta {
                if !in_capture {
                    let k = killer_key(mv);
                    if self.killers[ply][0] != k {
                        self.killers[ply][1] = self.killers[ply][0];
                        self.killers[ply][0] = k;
                    }
                    let slot = &mut self.history[side * 1024 + k as usize];
                    *slot = (*slot + depth * depth).min(400_000_000);
                }
                break;
            }
        }
        self.path.pop();
        if self.aborted {
            return 0;
        }

        let bound = if best >= beta {
            BOUND_LOWER
        } else if best > original_alpha {
            BOUND_EXACT
        } else {
            BOUND_UPPER
        };
        self.table
            .store(key, depth, to_table(best, ply), bound, best_index);
        best
    }
}

/// Ищет лучший ход в состоянии `state`.
///
/// `history` — позиции ветки партии, предшествующие текущей (нужны для учёта повторений).
/// `stop` позволяет прервать поиск извне.
pub fn search(
    state: &GameState,
    history: &[Position],
    params: &SearchParams,
    stop: &AtomicBool,
) -> SearchResult {
    let mut rng = Rng::new(params.seed);
    let mut root_moves = legal_moves(&state.pos);
    dedupe_captures(&mut root_moves);
    let mut root: Vec<(Move, i32)> = root_moves.into_iter().map(|m| (m, 0)).collect();
    let mut result = SearchResult {
        best: None,
        score: 0,
        depth: 0,
        nodes: 0,
        root_scores: Vec::new(),
        aborted: false,
    };
    if root.is_empty() {
        return result;
    }
    // Случайный начальный порядок: из равноценных ходов выбирается случайный.
    rng.shuffle(&mut root);

    let mut path = Vec::with_capacity(history.len() + MAX_PLY + 1);
    path.extend_from_slice(history);
    let mut searcher = Searcher {
        table: Table::new(params.table_bits),
        stop,
        started: Instant::now(),
        hard_time: params.hard_time,
        nodes: 0,
        work: 0,
        aborted: false,
        root_index: path.len(),
        path,
        killers: vec![[NO_KILLER; 2]; MAX_PLY + 1],
        history: vec![0; 2 * 1024],
        move_buffers: (0..=MAX_PLY).map(|_| Vec::with_capacity(32)).collect(),
        order_buffers: (0..=MAX_PLY).map(|_| Vec::with_capacity(32)).collect(),
        quiescence: params.quiescence,
        jitter: params.jitter.max(0),
        seed: rng.next_u64(),
    };

    let single = root.len() == 1;
    let max_depth = params.max_depth.clamp(1, MAX_PLY as i32 - 2);
    for depth in 1..=max_depth {
        let mut alpha = -INF;
        let mut best: Option<(usize, i32)> = None;
        searcher.path.push(state.pos);
        // Индексный цикл: оценки записываются в `root` по ходу перебора.
        #[allow(clippy::needless_range_loop)]
        for i in 0..root.len() {
            let mv = root[i].0;
            let child = state.play(&mv);
            let next_depth = if mv.is_capture() || single {
                depth
            } else {
                depth - 1
            };
            let score = if i == 0 || params.exact_root_scores {
                -searcher.negamax(&child, next_depth, -INF, -alpha, 1, true)
            } else {
                let mut s = -searcher.negamax(&child, next_depth, -alpha - 1, -alpha, 1, false);
                if s > alpha && !searcher.aborted {
                    s = -searcher.negamax(&child, next_depth, -INF, -alpha, 1, true);
                }
                s
            };
            if searcher.aborted {
                break;
            }
            root[i].1 = score;
            if best.is_none_or(|(_, b)| score > b) {
                best = Some((i, score));
            }
            if !params.exact_root_scores && score > alpha {
                alpha = score;
            }
        }
        searcher.path.pop();

        // Прерванная итерация учитывается, только если в ней уже оценён хотя бы первый
        // (лучший по прошлой итерации) ход: найденное в ней улучшение достоверно.
        if let Some((index, score)) = best {
            result.best = Some(root[index].0);
            result.score = score;
            root[..=index].rotate_right(1);
        }
        if searcher.aborted {
            result.aborted = true;
            break;
        }
        result.depth = depth;
        result.root_scores = root.clone();

        // Форсированный исход найден, и глубины хватает, чтобы в нём не сомневаться.
        if result.score.abs() >= WIN_BOUND && depth >= WIN - result.score.abs() + 2 {
            break;
        }
        if let Some(soft) = params.soft_time {
            if searcher.started.elapsed() >= soft {
                break;
            }
        }
    }
    result.nodes = searcher.nodes;
    if result.root_scores.is_empty() {
        result.root_scores = root;
    }
    result
}
