//! Эталон генератора ходов: число последовательностей ходов из начальной позиции.
//!
//! Числа проверены независимо. Если тест не сходится — ошибка в генераторе;
//! менять числа нельзя.

use rucheckers::board::Position;
use rucheckers::moves::perft;

/// Глубины 1–9. Взятия с разным маршрутом считаются разными ходами.
const EXPECTED: [u64; 9] = [7, 49, 302, 1469, 7482, 37986, 190146, 929905, 4570667];

#[test]
fn perft_from_initial_position_depths_1_to_9() {
    let pos = Position::initial();
    for (i, &expected) in EXPECTED.iter().enumerate() {
        let depth = i as u32 + 1;
        assert_eq!(perft(&pos, depth), expected, "perft({depth})");
    }
}
