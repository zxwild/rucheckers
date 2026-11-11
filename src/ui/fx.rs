//! Электрический разряд при взятии нескольких шашек: молния, вспышки и искры.
//!
//! Разряд выходит из бьющей шашки, когда она встала на конечное поле, и по цепи
//! проходит через побитые шашки, начиная с ближайшей — взятой последней. Каждая шашка
//! при ударе вспыхивает, рассыпается искрами и исчезает.
//!
//! Модуль только считает кадр: ломаные молнии, вспышки и положения искр. Он не знает
//! об окне и не хранит состояния между кадрами — кадр полностью определяется временем
//! и зерном, поэтому эффект воспроизводим и проверяется тестами. На экран его выводит
//! графическая часть средствами той же аппаратно ускоренной отрисовки, что и всю доску.
//!
//! Координаты — экранные, в клетках: единица равна стороне клетки, ось Y направлена
//! вниз, начало — левый верхний угол игрового поля.

use crate::rng::Rng;
use std::f32::consts::TAU;

/// Наименьшее число побитых за ход шашек, при котором показывается разряд.
pub const MIN_CAPTURES: u32 = 2;
/// Пауза между приходом бьющей шашки на конечное поле и началом разряда, с.
pub const STRIKE_DELAY: f64 = 0.04;
/// Задержка между ударами по соседним шашкам цепи, с.
pub const STRIKE_INTERVAL: f64 = 0.065;
/// Время исчезновения шашки после удара, с.
pub const VANISH_SECONDS: f64 = 0.12;
/// Время, которое молния держится после последнего удара, с.
pub const BOLT_HOLD_SECONDS: f64 = 0.24;
/// Время затухания молнии в конце, с.
pub const BOLT_FADE_SECONDS: f64 = 0.10;
/// Длина самого мелкого звена ломаной молнии, клеток (приблизительно).
pub const BOLT_FINE_SEGMENT: f32 = 0.16;
/// Размах самых мелких зубцов в долях длины мелкого звена.
pub const BOLT_FINE_BEND: f32 = 0.34;
/// Во сколько раз размах зубцов меньше размаха изгибов предыдущего, более крупного уровня.
pub const BOLT_ROUGHNESS: f32 = 0.62;
/// Предел размаха крупных изгибов, клеток: молния не уходит далеко от прямой.
pub const BOLT_MAX_BEND: f32 = 0.16;
/// Период, с которым молния меняет форму (мерцание), с.
pub const FLICKER_SECONDS: f64 = 0.045;
/// Длительность вспышки на месте удара, с.
pub const FLASH_SECONDS: f64 = 0.18;
/// Число искр от одной побитой шашки.
pub const SPARKS_PER_PIECE: usize = 28;
/// Время жизни искры, с: от и до.
pub const SPARK_LIFE: (f32, f32) = (0.35, 0.90);
/// Начальная скорость искры, клеток в секунду: от и до.
pub const SPARK_SPEED: (f32, f32) = (1.4, 5.6);
/// Сопротивление воздуха: скорость искры убывает в e раз за `1 / SPARK_DRAG` секунд.
pub const SPARK_DRAG: f32 = 3.2;
/// Ускорение, с которым искры падают вниз по экрану, клеток/с².
pub const SPARK_GRAVITY: f32 = 5.5;
/// Длина светящегося следа искры во времени, с.
pub const SPARK_TRAIL_SECONDS: f32 = 0.045;

/// Цвет сердцевины молнии: почти белый с зеленовато-жёлтым оттенком.
pub const BOLT_CORE: [f32; 3] = [0.93, 1.00, 0.70];
/// Цвет свечения молнии и искр: зеленовато-жёлтый.
pub const GLOW: [f32; 3] = [0.66, 0.95, 0.08];

/// Точка в экранных координатах доски (в клетках).
pub type Point = (f32, f32);

/// Момент удара по шашке с номером `index` в цепи (0 — ближайшая к источнику)
/// относительно начала разряда, с.
pub fn strike_offset(index: usize) -> f64 {
    index as f64 * STRIKE_INTERVAL
}

/// Время от начала разряда до исчезновения последней из `count` шашек, с.
pub fn chain_seconds(count: usize) -> f64 {
    strike_offset(count.saturating_sub(1)) + VANISH_SECONDS
}

/// Время от начала разряда до угасания молнии, с. Искры летят ещё некоторое время.
pub fn bolt_seconds(count: usize) -> f64 {
    strike_offset(count.saturating_sub(1)) + BOLT_HOLD_SECONDS
}

/// Цвет искры по её «температуре»: 1 — только что вылетела (почти белая),
/// 0 — догорает (тёмно-жёлтая). В середине жизни искра зеленовато-жёлтая.
pub fn spark_color(heat: f32) -> [f32; 3] {
    const HOT: [f32; 3] = [0.95, 1.00, 0.74];
    const COOL: [f32; 3] = [0.46, 0.60, 0.04];
    let heat = heat.clamp(0.0, 1.0);
    let (from, to, t) = if heat > 0.6 {
        (GLOW, HOT, (heat - 0.6) / 0.4)
    } else {
        (COOL, GLOW, heat / 0.6)
    };
    [
        from[0] + (to[0] - from[0]) * t,
        from[1] + (to[1] - from[1]) * t,
        from[2] + (to[2] - from[2]) * t,
    ]
}

/// Положение искры через `age` секунд после вылета из точки `origin` со скоростью
/// `velocity` (клеток/с).
///
/// На искру действуют сопротивление воздуха и сила тяжести: `dv/dt = −k·v + g`.
/// Используется точное решение этого уравнения, поэтому кадр не зависит от частоты
/// обновления экрана.
pub fn spark_position(origin: Point, velocity: Point, age: f32) -> Point {
    let age = age.max(0.0);
    let k = SPARK_DRAG;
    let decay = (1.0 - (-k * age).exp()) / k;
    let fall = SPARK_GRAVITY * (age - decay) / k;
    (
        origin.0 + velocity.0 * decay,
        origin.1 + velocity.1 * decay + fall,
    )
}

/// Скорость искры через `age` секунд после вылета.
pub fn spark_velocity(velocity: Point, age: f32) -> Point {
    let fade = (-SPARK_DRAG * age.max(0.0)).exp();
    (
        velocity.0 * fade,
        velocity.1 * fade + SPARK_GRAVITY / SPARK_DRAG * (1.0 - fade),
    )
}

/// Размахи сдвига середин для молнии длины `length`: от самого крупного изгиба
/// к самым мелким зубцам. Число уровней подбирается так, чтобы мелкое звено было
/// длиной около [`BOLT_FINE_SEGMENT`], поэтому короткие и длинные молнии изломаны
/// одинаково мелко.
fn bolt_amplitudes(length: f32) -> Vec<f32> {
    let levels = ((length / BOLT_FINE_SEGMENT).max(1.0).log2().ceil() as usize).clamp(1, 7);
    let fine = BOLT_FINE_BEND * length / (1u32 << levels) as f32;
    (0..levels)
        .map(|level| (fine / BOLT_ROUGHNESS.powi((levels - 1 - level) as i32)).min(BOLT_MAX_BEND))
        .collect()
}

/// Ломаная молнии от `a` до `b`: отрезок, изломанный смещением середин.
///
/// Каждый проход делит все звенья пополам и сдвигает середины поперёк общего
/// направления; размах сдвига от прохода к проходу уменьшается. Концы остаются
/// на месте. Одинаковое зерно даёт одинаковую ломаную.
pub fn bolt_path(a: Point, b: Point, seed: u64) -> Vec<Point> {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length = dx.hypot(dy);
    if !length.is_finite() || length < 1e-4 {
        return vec![a, b];
    }
    // Единичная нормаль к направлению молнии.
    let normal = (-dy / length, dx / length);
    let mut rng = Rng::new(seed);
    let mut points = vec![a, b];
    for amplitude in bolt_amplitudes(length) {
        let mut next = Vec::with_capacity(points.len() * 2 - 1);
        for pair in points.windows(2) {
            let shift = (rng.unit() as f32 * 2.0 - 1.0) * amplitude;
            let middle = (
                (pair[0].0 + pair[1].0) / 2.0 + normal.0 * shift,
                (pair[0].1 + pair[1].1) / 2.0 + normal.1 * shift,
            );
            next.push(pair[0]);
            next.push(middle);
        }
        next.push(b);
        points = next;
    }
    points
}

/// Наибольшее отклонение точек ломаной от прямой, которое может дать [`bolt_path`]
/// для отрезка длины `length`.
pub fn max_bolt_deviation(length: f32) -> f32 {
    if !length.is_finite() || length < 1e-4 {
        return 0.0;
    }
    bolt_amplitudes(length).iter().sum()
}

/// Ветвь молнии на кадре.
#[derive(Clone, PartialEq, Debug)]
pub struct Bolt {
    /// Вершины ломаной.
    pub points: Vec<Point>,
    /// Яркость 0…1.
    pub intensity: f32,
    /// Относительная толщина: 1 — основной канал, меньше — боковое ответвление.
    pub width: f32,
}

/// Вспышка на месте удара.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Flash {
    pub center: Point,
    /// Радиус в клетках.
    pub radius: f32,
    /// Яркость 0…1.
    pub intensity: f32,
}

/// Искра на кадре: короткий светящийся штрих от хвоста к голове.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Spark {
    pub head: Point,
    pub tail: Point,
    /// Яркость 0…1.
    pub intensity: f32,
    /// «Температура» 1…0 — см. [`spark_color`].
    pub heat: f32,
    /// Относительная толщина штриха.
    pub size: f32,
}

/// Кадр разряда.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct DischargeFrame {
    pub bolts: Vec<Bolt>,
    pub flashes: Vec<Flash>,
    pub sparks: Vec<Spark>,
}

impl DischargeFrame {
    /// Нечего рисовать.
    pub fn is_empty(&self) -> bool {
        self.bolts.is_empty() && self.flashes.is_empty() && self.sparks.is_empty()
    }
}

/// Параметры одной искры, вычисленные из зерна.
struct SparkSeed {
    origin: Point,
    velocity: Point,
    delay: f32,
    life: f32,
    size: f32,
}

/// Разряд: источник (бьющая шашка), цепь побитых шашек и время начала.
#[derive(Clone, PartialEq, Debug)]
pub struct Discharge {
    source: Point,
    targets: Vec<Point>,
    start: f64,
    seed: u64,
}

impl Discharge {
    /// Разряд из точки `source` по цепи `targets` (в порядке ударов), начинающийся
    /// в момент `start`. `seed` определяет форму молний и разлёт искр.
    pub fn new(source: Point, targets: Vec<Point>, start: f64, seed: u64) -> Discharge {
        Discharge {
            source,
            targets,
            start,
            seed,
        }
    }

    /// Момент начала разряда.
    pub fn start(&self) -> f64 {
        self.start
    }

    /// Число шашек в цепи.
    pub fn len(&self) -> usize {
        self.targets.len()
    }

    /// Цепь пуста: разряду некуда бить.
    pub fn is_empty(&self) -> bool {
        self.targets.is_empty()
    }

    /// Момент удара по шашке с номером `index` в цепи.
    pub fn strike_time(&self, index: usize) -> f64 {
        self.start + strike_offset(index)
    }

    /// Момент, когда гаснет молния.
    pub fn bolt_end(&self) -> f64 {
        self.start + bolt_seconds(self.targets.len())
    }

    /// Момент, когда гаснет последняя искра и эффект заканчивается.
    pub fn end(&self) -> f64 {
        let sparks = self.strike_time(self.targets.len().saturating_sub(1)) + f64::from(SPARK_LIFE.1) + 0.05;
        sparks.max(self.bolt_end())
    }

    /// Закончился ли эффект к моменту `now`.
    pub fn is_finished(&self, now: f64) -> bool {
        self.targets.is_empty() || now >= self.end()
    }

    /// Зерно для части эффекта: не зависит от порядка вычислений.
    fn sub_seed(&self, a: u64, b: u64, c: u64) -> u64 {
        let mut rng = Rng::new(
            self.seed
                ^ a.wrapping_mul(0x9E37_79B9_7F4A_7C15)
                ^ b.wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
                ^ c.wrapping_mul(0x1656_67B1_9E37_79F9),
        );
        rng.next_u64()
    }

    /// Начало и конец звена цепи с номером `index`.
    fn link(&self, index: usize) -> (Point, Point) {
        let from = if index == 0 {
            self.source
        } else {
            self.targets[index - 1]
        };
        (from, self.targets[index])
    }

    fn spark_seed(&self, target: usize, number: usize) -> SparkSeed {
        let mut rng = Rng::new(self.sub_seed(1, target as u64, number as u64));
        let angle = rng.unit() as f32 * TAU;
        let quick = rng.unit() as f32;
        let speed = SPARK_SPEED.0 + (SPARK_SPEED.1 - SPARK_SPEED.0) * quick * quick;
        let (sin, cos) = angle.sin_cos();
        let center = self.targets[target];
        SparkSeed {
            // Искры вылетают не из одной точки, а с небольшого круга внутри шашки.
            origin: (center.0 + cos * 0.10, center.1 + sin * 0.10),
            velocity: (cos * speed, sin * speed - 0.9),
            delay: rng.unit() as f32 * 0.035,
            life: SPARK_LIFE.0 + (SPARK_LIFE.1 - SPARK_LIFE.0) * rng.unit() as f32,
            size: 0.55 + 0.45 * rng.unit() as f32,
        }
    }

    /// Кадр эффекта в момент `now`.
    pub fn frame(&self, now: f64) -> DischargeFrame {
        let mut frame = DischargeFrame::default();
        if self.targets.is_empty() || now < self.start || now >= self.end() {
            return frame;
        }

        // Молния: звенья цепи появляются по очереди и мерцают, пока разряд не погаснет.
        let bolt_end = self.bolt_end();
        if now < bolt_end {
            let fade = ((bolt_end - now) / BOLT_FADE_SECONDS).clamp(0.0, 1.0) as f32;
            for index in 0..self.targets.len() {
                let struck = self.strike_time(index);
                if now < struck {
                    break;
                }
                let phase = ((now - struck) / FLICKER_SECONDS).floor() as u64;
                let mut rng = Rng::new(self.sub_seed(2, index as u64, phase));
                let flicker = 0.62 + 0.38 * rng.unit() as f32;
                // В момент удара молния ярче всего.
                let fresh = (1.0 - (now - struck) / 0.09).clamp(0.0, 1.0) as f32;
                let intensity = (fade * (flicker + 0.4 * fresh)).min(1.0);
                let (from, to) = self.link(index);
                let main = bolt_path(from, to, rng.next_u64());

                // Боковые ответвления от случайных вершин основного канала.
                let forks = 1 + rng.below(2);
                for _ in 0..forks {
                    let vertex = main[1 + rng.below(main.len() - 2)];
                    let along = (to.1 - from.1).atan2(to.0 - from.0);
                    let turn = (0.45 + 0.55 * rng.unit() as f32) * if rng.chance(0.5) { 1.0 } else { -1.0 };
                    let length = 0.22 + 0.38 * rng.unit() as f32;
                    let tip = (
                        vertex.0 + (along + turn).cos() * length,
                        vertex.1 + (along + turn).sin() * length,
                    );
                    frame.bolts.push(Bolt {
                        points: bolt_path(vertex, tip, rng.next_u64()),
                        intensity: intensity * 0.55,
                        width: 0.5,
                    });
                }
                frame.bolts.push(Bolt {
                    points: main,
                    intensity,
                    width: 1.0,
                });
            }
        }

        // Вспышки: у источника в начале разряда и на каждой шашке в момент удара.
        let mut flash = |center: Point, at: f64, scale: f32| {
            let age = now - at;
            if (0.0..FLASH_SECONDS).contains(&age) {
                let u = (age / FLASH_SECONDS) as f32;
                frame.flashes.push(Flash {
                    center,
                    radius: scale * (0.28 + 0.44 * u),
                    intensity: (1.0 - u) * (1.0 - u),
                });
            }
        };
        flash(self.source, self.start, 0.8);
        for (index, &target) in self.targets.iter().enumerate() {
            flash(target, self.strike_time(index), 1.0);
        }

        // Искры.
        for target in 0..self.targets.len() {
            let struck = self.strike_time(target);
            if now < struck {
                break;
            }
            for number in 0..SPARKS_PER_PIECE {
                let spark = self.spark_seed(target, number);
                let age = (now - struck) as f32 - spark.delay;
                if age < 0.0 || age >= spark.life {
                    continue;
                }
                let left = 1.0 - age / spark.life;
                frame.sparks.push(Spark {
                    head: spark_position(spark.origin, spark.velocity, age),
                    tail: spark_position(spark.origin, spark.velocity, (age - SPARK_TRAIL_SECONDS).max(0.0)),
                    intensity: left.powf(0.8),
                    heat: left,
                    size: spark.size,
                });
            }
        }
        frame
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn distance(a: Point, b: Point) -> f32 {
        (a.0 - b.0).hypot(a.1 - b.1)
    }

    /// Расстояние от точки до прямой, проходящей через `a` и `b`.
    fn deviation(p: Point, a: Point, b: Point) -> f32 {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        ((p.0 - a.0) * dy - (p.1 - a.1) * dx).abs() / dx.hypot(dy)
    }

    fn sample() -> Discharge {
        // Источник и три побитые шашки по диагонали.
        Discharge::new((6.5, 1.5), vec![(5.5, 2.5), (3.5, 4.5), (1.5, 6.5)], 10.0, 42)
    }

    #[test]
    fn bolt_connects_its_ends_and_stays_near_the_straight_line() {
        // Короткая, средняя и длинная молнии, в том числе по диагонали через всю доску.
        let ends = [
            ((1.5, 6.5), (2.5, 5.5)),
            ((1.5, 6.5), (4.5, 3.5)),
            ((0.5, 7.5), (7.5, 0.5)),
            ((6.5, 2.5), (6.9, 2.6)),
        ];
        for (a, b) in ends {
            let straight = distance(a, b);
            let limit = max_bolt_deviation(straight) + 1e-4;
            assert!(limit < 0.9, "молния не уходит от прямой дальше клетки: {limit}");
            for seed in 0..200u64 {
                let path = bolt_path(a, b, seed);
                assert_eq!(path[0], a);
                assert_eq!(path[path.len() - 1], b);
                assert!((path.len() - 1).is_power_of_two());
                for &p in &path {
                    assert!(p.0.is_finite() && p.1.is_finite());
                    assert!(
                        deviation(p, a, b) <= limit,
                        "зерно {seed}: точка {p:?} ушла далеко"
                    );
                }
                // Звенья мелкие, а сама молния изломана, а не прямая. Короткое боковое
                // ответвление из двух-четырёх звеньев может случайно выйти почти прямым,
                // поэтому излом проверяется у молний между шашками (не короче клетки).
                let length: f32 = path.windows(2).map(|w| distance(w[0], w[1])).sum();
                assert!(length >= straight);
                if straight >= 1.0 {
                    assert!(
                        length > straight * 1.015,
                        "зерно {seed}: {} от {straight}",
                        length / straight
                    );
                }
                let longest = path.windows(2).map(|w| distance(w[0], w[1])).fold(0.0, f32::max);
                assert!(longest < 2.5 * BOLT_FINE_SEGMENT, "зерно {seed}: звено {longest}");
            }
        }
    }

    #[test]
    fn bolt_shape_depends_only_on_the_seed() {
        let (a, b) = ((0.5, 0.5), (3.5, 3.5));
        assert_eq!(bolt_path(a, b, 7), bolt_path(a, b, 7));
        assert_ne!(bolt_path(a, b, 7), bolt_path(a, b, 8));
        // Число звеньев растёт с длиной, но ограничено.
        assert_eq!(bolt_path(a, b, 1).len(), 33);
        assert_eq!(bolt_path((0.0, 0.0), (0.1, 0.0), 1).len(), 3);
        assert_eq!(bolt_path((0.0, 0.0), (1000.0, 0.0), 1).len(), 129);
        // Вырожденные случаи не дают ни сбоя, ни NaN.
        assert_eq!(bolt_path(a, a, 1), vec![a, a]);
        assert_eq!(bolt_path((f32::NAN, 0.0), b, 1).len(), 2);
        assert_eq!(bolt_path(a, (f32::INFINITY, 0.0), 1).len(), 2);
        assert_eq!(max_bolt_deviation(0.0), 0.0);
        assert_eq!(max_bolt_deviation(f32::NAN), 0.0);
    }

    #[test]
    fn strikes_run_down_the_chain_in_order() {
        let fx = sample();
        assert_eq!(fx.len(), 3);
        assert!(!fx.is_empty());
        assert_eq!(fx.strike_time(0), 10.0);
        assert!((fx.strike_time(1) - (10.0 + STRIKE_INTERVAL)).abs() < 1e-12);
        assert!((fx.strike_time(2) - (10.0 + 2.0 * STRIKE_INTERVAL)).abs() < 1e-12);
        assert!((chain_seconds(3) - (2.0 * STRIKE_INTERVAL + VANISH_SECONDS)).abs() < 1e-12);
        assert_eq!(chain_seconds(0), VANISH_SECONDS);

        // До начала — ничего.
        assert!(fx.frame(9.999).is_empty());
        assert!(!fx.is_finished(9.0));
        // Сразу после начала: одно звено молнии (и его ответвления), вспышки у источника
        // и у первой шашки, искры только от первой шашки.
        let main_bolts = |f: &DischargeFrame| f.bolts.iter().filter(|b| b.width == 1.0).count();
        let first = fx.frame(10.0 + 0.5 * STRIKE_INTERVAL);
        assert_eq!(main_bolts(&first), 1);
        assert_eq!(first.flashes.len(), 2);
        assert!(!first.sparks.is_empty());
        assert!(first.sparks.iter().all(|s| distance(s.head, (5.5, 2.5)) < 1.0));
        // После второго удара — два звена, после третьего — три.
        assert_eq!(main_bolts(&fx.frame(10.0 + 1.5 * STRIKE_INTERVAL)), 2);
        assert_eq!(main_bolts(&fx.frame(10.0 + 2.5 * STRIKE_INTERVAL)), 3);
        // Основные каналы соединяют соседние точки цепи.
        let full = fx.frame(10.0 + 2.5 * STRIKE_INTERVAL);
        let mains: Vec<&Bolt> = full.bolts.iter().filter(|b| b.width == 1.0).collect();
        let chain = [(6.5, 1.5), (5.5, 2.5), (3.5, 4.5), (1.5, 6.5)];
        for (i, bolt) in mains.iter().enumerate() {
            assert_eq!(bolt.points[0], chain[i]);
            assert_eq!(bolt.points[bolt.points.len() - 1], chain[i + 1]);
        }
    }

    #[test]
    fn bolt_flickers_then_goes_out_and_sparks_outlive_it() {
        let fx = sample();
        let at = |t: f64| fx.frame(t);
        // Форма молнии меняется от такта к такту, но внутри такта постоянна.
        let shapes = |t: f64| at(t).bolts.into_iter().map(|b| b.points).collect::<Vec<_>>();
        let t = fx.strike_time(2) + 0.01;
        assert_eq!(shapes(t), shapes(t + 0.2 * FLICKER_SECONDS));
        assert_ne!(shapes(t), shapes(t + 1.5 * FLICKER_SECONDS));
        // Яркость в допустимых пределах.
        for step in 0..200 {
            let frame = at(10.0 + f64::from(step) * 0.007);
            for bolt in &frame.bolts {
                assert!((0.0..=1.0).contains(&bolt.intensity));
                assert!(bolt.points.len() >= 2);
            }
            for flash in &frame.flashes {
                assert!((0.0..=1.0).contains(&flash.intensity) && flash.radius > 0.0 && flash.radius < 1.0);
            }
            for spark in &frame.sparks {
                assert!((0.0..=1.0).contains(&spark.intensity) && (0.0..=1.0).contains(&spark.heat));
            }
        }
        // Молния гаснет, искры ещё летят, потом эффект заканчивается.
        let after_bolt = at(fx.bolt_end() + 0.01);
        assert!(after_bolt.bolts.is_empty());
        assert!(!after_bolt.sparks.is_empty());
        assert!(fx.bolt_end() < fx.end());
        assert!(!fx.is_finished(fx.end() - 0.01));
        assert!(fx.is_finished(fx.end()));
        assert!(at(fx.end()).is_empty());
        assert!(at(fx.end() + 5.0).is_empty());
    }

    #[test]
    fn every_struck_piece_throws_sparks() {
        let fx = sample();
        // Через мгновение после последнего удара летят искры от всех трёх шашек.
        let frame = fx.frame(fx.strike_time(2) + 0.06);
        let targets = [(5.5, 2.5), (3.5, 4.5), (1.5, 6.5)];
        for target in targets {
            let near = frame
                .sparks
                .iter()
                .filter(|s| distance(s.head, target) < 0.9)
                .count();
            assert!(near >= SPARKS_PER_PIECE / 2, "у шашки {target:?} искр: {near}");
        }
        assert!(frame.sparks.len() <= 3 * SPARKS_PER_PIECE);
        // Искры разлетаются во все стороны, а не в одну.
        let (mut left, mut right, mut up, mut down) = (0, 0, 0, 0);
        for spark in frame.sparks.iter().filter(|s| distance(s.head, targets[2]) < 0.9) {
            left += usize::from(spark.head.0 < targets[2].0);
            right += usize::from(spark.head.0 > targets[2].0);
            up += usize::from(spark.head.1 < targets[2].1);
            down += usize::from(spark.head.1 > targets[2].1);
        }
        assert!(
            left >= 4 && right >= 4 && up >= 4 && down >= 4,
            "{left} {right} {up} {down}"
        );
        // Никакая искра не улетает далеко за пределы своего удара.
        for step in 0..150 {
            let frame = fx.frame(10.0 + f64::from(step) * 0.01);
            for spark in &frame.sparks {
                let nearest = targets
                    .iter()
                    .map(|&t| distance(spark.head, t))
                    .fold(f32::MAX, f32::min);
                assert!(nearest < 3.0, "искра улетела на {nearest} клеток");
                assert!(distance(spark.head, spark.tail) < 0.4, "след искры короткий");
            }
        }
    }

    #[test]
    fn sparks_move_with_drag_and_gravity() {
        let origin = (2.0, 3.0);
        let velocity = (3.0, -2.5);
        assert_eq!(spark_position(origin, velocity, 0.0), origin);
        assert_eq!(spark_velocity(velocity, 0.0), velocity);
        assert_eq!(
            spark_position(origin, velocity, -1.0),
            origin,
            "до вылета искра на месте"
        );

        // Точное решение совпадает с пошаговым интегрированием dv/dt = −k·v + g.
        let (mut p, mut v) = (origin, velocity);
        let dt = 1e-4f32;
        for step in 1..=10_000 {
            let acceleration = (-SPARK_DRAG * v.0, -SPARK_DRAG * v.1 + SPARK_GRAVITY);
            p = (p.0 + v.0 * dt, p.1 + v.1 * dt);
            v = (v.0 + acceleration.0 * dt, v.1 + acceleration.1 * dt);
            if step % 2_000 == 0 {
                let age = step as f32 * dt;
                let exact = spark_position(origin, velocity, age);
                let exact_v = spark_velocity(velocity, age);
                assert!(distance(p, exact) < 2e-3, "t = {age}: {p:?} и {exact:?}");
                assert!(distance(v, exact_v) < 2e-3, "t = {age}: {v:?} и {exact_v:?}");
            }
        }
        // Горизонтальная скорость гаснет, вертикальная стремится к установившейся скорости падения.
        let late = spark_velocity(velocity, 5.0);
        assert!(late.0.abs() < 1e-3);
        assert!((late.1 - SPARK_GRAVITY / SPARK_DRAG).abs() < 1e-3);
        // Искра, вылетевшая вверх, сначала поднимается, потом падает ниже точки вылета.
        let up = (0.0, -3.0);
        assert!(spark_position(origin, up, 0.15).1 < origin.1);
        assert!(spark_position(origin, up, 2.0).1 > origin.1);
    }

    #[test]
    fn colours_are_greenish_yellow() {
        /// Цветовой тон в градусах: 60 — жёлтый, 120 — зелёный.
        fn hue(c: [f32; 3]) -> f32 {
            let max = c[0].max(c[1]).max(c[2]);
            let min = c[0].min(c[1]).min(c[2]);
            assert!(max - min > 0.05, "цвет не серый: {c:?}");
            let h = if max == c[0] {
                (c[1] - c[2]) / (max - min)
            } else if max == c[1] {
                2.0 + (c[2] - c[0]) / (max - min)
            } else {
                4.0 + (c[0] - c[1]) / (max - min)
            };
            (60.0 * h).rem_euclid(360.0)
        }
        // Свечение: жёлтый с зелёным оттенком — между жёлтым (60°) и жёлто-зелёным (90°).
        assert!((66.0..=85.0).contains(&hue(GLOW)), "{}", hue(GLOW));
        for step in 0..=20 {
            let heat = step as f32 / 20.0;
            let c = spark_color(heat);
            assert!(c.iter().all(|v| (0.0..=1.0).contains(v)));
            assert!(
                (64.0..=85.0).contains(&hue(c)),
                "температура {heat}: тон {}",
                hue(c)
            );
            assert!(
                c[1] >= c[0] && c[0] > c[2],
                "зелёная составляющая не меньше красной: {c:?}"
            );
        }
        // Горячая искра светлее остывшей.
        let brightness = |c: [f32; 3]| c[0] + c[1] + c[2];
        assert!(brightness(spark_color(1.0)) > brightness(spark_color(0.5)));
        assert!(brightness(spark_color(0.5)) > brightness(spark_color(0.0)));
        assert_eq!(spark_color(5.0), spark_color(1.0));
        assert_eq!(spark_color(-5.0), spark_color(0.0));
        assert!((64.0..=85.0).contains(&hue(BOLT_CORE)));
    }

    #[test]
    fn effect_is_reproducible_and_seed_dependent() {
        let a = sample();
        let b = sample();
        let c = Discharge::new((6.5, 1.5), vec![(5.5, 2.5), (3.5, 4.5), (1.5, 6.5)], 10.0, 43);
        for step in 0..40 {
            let t = 10.0 + f64::from(step) * 0.03;
            assert_eq!(a.frame(t), b.frame(t));
        }
        assert_ne!(a.frame(10.1), c.frame(10.1));
        // Пустая цепь и вырожденные точки не вызывают сбоя.
        let empty = Discharge::new((1.0, 1.0), Vec::new(), 0.0, 1);
        assert!(empty.is_empty() && empty.is_finished(0.0) && empty.frame(0.1).is_empty());
        let same = Discharge::new((1.0, 1.0), vec![(1.0, 1.0), (1.0, 1.0)], 0.0, 1);
        for step in 0..60 {
            let frame = same.frame(f64::from(step) * 0.02);
            for bolt in &frame.bolts {
                assert!(bolt.points.iter().all(|p| p.0.is_finite() && p.1.is_finite()));
            }
        }
    }
}
