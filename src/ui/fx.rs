//! Электрический разряд при взятии нескольких шашек: молния, вспышки и искры.
//!
//! Молния тянется за бьющей шашкой по её пути и появляется одновременно с движением:
//! шашка летит, разряд следует за ней и постепенно гаснет позади. Пролетая над побитой
//! шашкой, разряд поражает её: шашка вспыхивает, рассыпается искрами и исчезает.
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
/// Через сколько секунд после пролёта шашки молния на этом участке пути гаснет.
pub const TRAIL_SECONDS: f64 = 0.45;
/// Сколько секунд после пролёта шашки участок молнии светится в полную силу.
pub const TRAIL_FULL_SECONDS: f64 = 0.10;
/// Время исчезновения шашки после удара, с.
pub const VANISH_SECONDS: f64 = 0.12;
/// Длина самого мелкого звена ломаной молнии, клеток (приблизительно).
pub const BOLT_FINE_SEGMENT: f32 = 0.16;
/// Размах самых мелких зубцов в долях длины мелкого звена.
pub const BOLT_FINE_BEND: f32 = 0.34;
/// Во сколько раз размах зубцов меньше размаха изгибов предыдущего, более крупного уровня.
pub const BOLT_ROUGHNESS: f32 = 0.62;
/// Предел размаха крупных изгибов, клеток: молния не уходит далеко от пути шашки.
pub const BOLT_MAX_BEND: f32 = 0.16;
/// Период, с которым молния меняет форму (мерцание), с.
pub const FLICKER_SECONDS: f64 = 0.045;
/// Длительность вспышки на месте удара, с.
pub const FLASH_SECONDS: f64 = 0.18;
/// Время, за которое гаснет свечение вокруг шашки после её остановки, с.
pub const HEAD_GLOW_FADE_SECONDS: f64 = 0.12;
/// Внешний радиус свечения вокруг летящей шашки, клеток (сама шашка — 0,40).
pub const HALO_RADIUS: f32 = 0.66;
/// Число искр от одной побитой шашки.
pub const SPARKS_PER_PIECE: usize = 28;
/// Время жизни искры, с: от и до.
pub const SPARK_LIFE: (f32, f32) = (0.35, 0.90);
/// Начальная скорость искры, клеток в секунду: от и до.
pub const SPARK_SPEED: (f32, f32) = (1.4, 5.6);
/// Добавка к скорости искр в направлении полёта шашки, клеток в секунду.
pub const SPARK_CARRY: f32 = 1.3;
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

/// Какие части эффекта показывать.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct EffectSet {
    /// Молния за шашкой.
    pub lightning: bool,
    /// Искры от побитых шашек.
    pub sparks: bool,
    /// Вспышки на месте удара и свечение вокруг летящей шашки.
    pub flashes: bool,
}

impl EffectSet {
    /// Все части включены.
    pub const ALL: Self = Self {
        lightning: true,
        sparks: true,
        flashes: true,
    };
    /// Все части выключены.
    pub const NONE: Self = Self {
        lightning: false,
        sparks: false,
        flashes: false,
    };

    /// Включена ли хотя бы одна часть.
    #[must_use]
    pub const fn any(self) -> bool {
        self.lightning || self.sparks || self.flashes
    }
}

/// Плавный разгон и торможение шашки на отрезке пути: доля пройденного расстояния
/// по доле прошедшего времени.
#[must_use]
pub fn ease(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// Обратная функция к [`ease`]: доля времени, за которую пройдена доля пути `u`.
#[must_use]
pub fn ease_inverse(u: f64) -> f64 {
    let u = u.clamp(0.0, 1.0);
    0.5 - ((1.0 - 2.0 * u).asin() / 3.0).sin()
}

/// Яркость участка молнии через `age` секунд после того, как над ним пролетела шашка.
#[must_use]
pub fn trail_intensity(age: f64) -> f32 {
    if !(0.0..TRAIL_SECONDS).contains(&age) {
        0.0
    } else if age <= TRAIL_FULL_SECONDS {
        1.0
    } else {
        (1.0 - ease((age - TRAIL_FULL_SECONDS) / (TRAIL_SECONDS - TRAIL_FULL_SECONDS))) as f32
    }
}

/// Цвет искры по её «температуре»: 1 — только что вылетела (почти белая),
/// 0 — догорает (тёмно-жёлтая). В середине жизни искра зеленовато-жёлтая.
#[must_use]
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
#[must_use]
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
#[must_use]
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
/// на месте, вершины идут вдоль отрезка равномерно. Одинаковое зерно даёт одинаковую
/// ломаную.
#[must_use]
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
                f32::midpoint(pair[0].0, pair[1].0) + normal.0 * shift,
                f32::midpoint(pair[0].1, pair[1].1) + normal.1 * shift,
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
#[must_use]
pub fn max_bolt_deviation(length: f32) -> f32 {
    if !length.is_finite() || length < 1e-4 {
        return 0.0;
    }
    bolt_amplitudes(length).iter().sum()
}

/// Участок молнии на кадре.
#[derive(Clone, PartialEq, Debug)]
pub struct Bolt {
    /// Вершины ломаной.
    pub points: Vec<Point>,
    /// Яркость 0…1.
    pub intensity: f32,
    /// Относительная толщина: 1 — основной канал, меньше — боковое ответвление.
    pub width: f32,
}

/// Вспышка или свечение.
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
    /// Молния за шашкой.
    pub bolts: Vec<Bolt>,
    /// Свечение вокруг летящей шашки: ореол, выступающий из-под неё.
    pub halo: Option<Flash>,
    /// Вспышки на побитых шашках.
    pub flashes: Vec<Flash>,
    pub sparks: Vec<Spark>,
}

impl DischargeFrame {
    /// Нечего рисовать.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bolts.is_empty() && self.halo.is_none() && self.flashes.is_empty() && self.sparks.is_empty()
    }
}

/// Удар разряда по побитой шашке.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Strike {
    /// Где стоит побитая шашка.
    pub center: Point,
    /// Момент удара: бьющая шашка пролетает над побитой.
    pub time: f64,
    /// Единичный вектор направления полёта бьющей шашки в этот момент.
    pub direction: Point,
}

/// Параметры одной искры, вычисленные из зерна.
struct SparkSeed {
    origin: Point,
    velocity: Point,
    delay: f32,
    life: f32,
    size: f32,
}

/// Число звеньев ломаной в одном участке молнии с общей яркостью.
const CHUNK: usize = 4;

/// Разряд, сопровождающий один ход.
#[derive(Clone, PartialEq, Debug)]
pub struct Discharge {
    /// Поля пути бьющей шашки и моменты, когда она на них находится.
    knots: Vec<(Point, f64)>,
    strikes: Vec<Strike>,
    seed: u64,
}

impl Discharge {
    /// Разряд по пути `knots` (точки и моменты времени, по порядку) с ударами `strikes`.
    /// `seed` определяет форму молний и разлёт искр.
    #[must_use]
    pub const fn new(knots: Vec<(Point, f64)>, strikes: Vec<Strike>, seed: u64) -> Self {
        Self { knots, strikes, seed }
    }

    /// Момент начала: шашка трогается с места.
    #[must_use]
    pub fn start(&self) -> f64 {
        self.knots.first().map_or(0.0, |k| k.1)
    }

    /// Момент, когда шашка встаёт на конечное поле.
    #[must_use]
    pub fn arrival(&self) -> f64 {
        self.knots.last().map_or(0.0, |k| k.1)
    }

    /// Удары по побитым шашкам.
    #[must_use]
    pub fn strikes(&self) -> &[Strike] {
        &self.strikes
    }

    /// Момент, когда гаснет молния.
    #[must_use]
    pub fn bolt_end(&self) -> f64 {
        self.arrival() + TRAIL_SECONDS
    }

    /// Момент, когда гаснет последняя искра и эффект заканчивается.
    pub fn end(&self) -> f64 {
        let last_strike = self.strikes.iter().map(|s| s.time).fold(self.start(), f64::max);
        (last_strike + f64::from(SPARK_LIFE.1) + 0.05).max(self.bolt_end())
    }

    /// Закончился ли эффект к моменту `now`.
    #[must_use]
    pub fn is_finished(&self, now: f64) -> bool {
        self.knots.len() < 2 || now >= self.end()
    }

    /// Положение бьющей шашки в момент `now`.
    #[must_use]
    pub fn head(&self, now: f64) -> Point {
        let Some(&(first, start)) = self.knots.first() else {
            return (0.0, 0.0);
        };
        if now <= start {
            // Отрезки, пройденные мгновенно (уже показанные прыжки), пропускаются.
            return self
                .knots
                .iter()
                .rev()
                .find(|k| k.1 <= start)
                .map_or(first, |k| k.0);
        }
        for pair in self.knots.windows(2) {
            let ((a, ta), (b, tb)) = (pair[0], pair[1]);
            if now < tb && tb > ta {
                let u = ease((now - ta) / (tb - ta)) as f32;
                return (a.0 + (b.0 - a.0) * u, a.1 + (b.1 - a.1) * u);
            }
        }
        self.knots[self.knots.len() - 1].0
    }

    /// Зерно для части эффекта: не зависит от порядка вычислений.
    const fn sub_seed(&self, a: u64, b: u64, c: u64) -> u64 {
        let mut rng = Rng::new(
            self.seed
                ^ a.wrapping_mul(0x9E37_79B9_7F4A_7C15)
                ^ b.wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
                ^ c.wrapping_mul(0x1656_67B1_9E37_79F9),
        );
        rng.next_u64()
    }

    fn spark_seed(&self, strike: usize, number: usize) -> SparkSeed {
        let mut rng = Rng::new(self.sub_seed(1, strike as u64, number as u64));
        let angle = rng.unit() as f32 * TAU;
        let quick = rng.unit() as f32;
        let speed = SPARK_SPEED.0 + (SPARK_SPEED.1 - SPARK_SPEED.0) * quick * quick;
        let (sin, cos) = angle.sin_cos();
        let hit = &self.strikes[strike];
        SparkSeed {
            // Искры вылетают не из одной точки, а с небольшого круга внутри шашки.
            origin: (hit.center.0 + cos * 0.10, hit.center.1 + sin * 0.10),
            // Разлёт во все стороны, с лёгким сносом по ходу шашки и вверх.
            velocity: (
                cos * speed + hit.direction.0 * SPARK_CARRY,
                sin * speed + hit.direction.1 * SPARK_CARRY - 0.9,
            ),
            delay: rng.unit() as f32 * 0.035,
            life: SPARK_LIFE.0 + (SPARK_LIFE.1 - SPARK_LIFE.0) * rng.unit() as f32,
            size: 0.55 + 0.45 * rng.unit() as f32,
        }
    }

    /// Молния на отрезке пути с номером `hop`: от его начала до шашки (или до конца
    /// отрезка, если шашка уже улетела дальше), разбитая на участки по давности.
    fn hop_bolts(&self, hop: usize, now: f64, phase: u64, flicker: f32, bolts: &mut Vec<Bolt>) {
        let ((a, ta), (b, tb)) = (self.knots[hop], self.knots[hop + 1]);
        if now < ta || now - tb >= TRAIL_SECONDS {
            return;
        }
        let instant = tb <= ta;
        let reached = if instant || now >= tb {
            1.0
        } else {
            ease((now - ta) / (tb - ta))
        };
        let full = bolt_path(a, b, self.sub_seed(2, hop as u64, phase));
        let links = full.len() - 1;
        let whole = ((reached * links as f64).floor() as usize).min(links);
        let mut visible: Vec<Point> = full[..=whole].to_vec();
        if reached < 1.0 {
            // Молния кончается точно у шашки.
            let u = reached as f32;
            visible.push((a.0 + (b.0 - a.0) * u, a.1 + (b.1 - a.1) * u));
        }
        if visible.len() < 2 {
            return;
        }
        // Когда шашка пролетела точку, лежащую на доле `along` отрезка.
        let passed = |along: f64| {
            if instant {
                ta
            } else {
                ta + (tb - ta) * ease_inverse(along.min(reached))
            }
        };

        let mut newest = 0.0f32;
        let mut index = 0;
        while index + 1 < visible.len() {
            let last = (index + CHUNK).min(visible.len() - 1);
            let middle = (index + last) as f64 / 2.0 / links as f64;
            let intensity = trail_intensity(now - passed(middle)) * flicker;
            if intensity > 0.02 {
                bolts.push(Bolt {
                    points: visible[index..=last].to_vec(),
                    intensity,
                    width: 1.0,
                });
            }
            newest = intensity;
            index = last;
        }

        // Боковое ответвление у самого свежего участка.
        if newest > 0.25 && visible.len() > 3 {
            let mut rng = Rng::new(self.sub_seed(3, hop as u64, phase));
            let vertex = visible[visible.len() - 2 - rng.below((visible.len() - 2).min(4))];
            let along = (b.1 - a.1).atan2(b.0 - a.0);
            // Ответвление отходит назад и вбок от направления полёта.
            let turn = (2.2 + 0.6 * rng.unit() as f32) * if rng.chance(0.5) { 1.0 } else { -1.0 };
            let length = 0.20 + 0.34 * rng.unit() as f32;
            let tip = (
                vertex.0 + (along + turn).cos() * length,
                vertex.1 + (along + turn).sin() * length,
            );
            bolts.push(Bolt {
                points: bolt_path(vertex, tip, rng.next_u64()),
                intensity: newest * 0.55,
                width: 0.5,
            });
        }
    }

    /// Кадр эффекта в момент `now`; показываются только части из `parts`.
    #[must_use]
    pub fn frame(&self, now: f64, parts: EffectSet) -> DischargeFrame {
        let mut frame = DischargeFrame::default();
        if self.knots.len() < 2 || now < self.start() || now >= self.end() {
            return frame;
        }

        // Молния: тянется за шашкой, мерцает и гаснет позади неё.
        let phase = ((now - self.start()) / FLICKER_SECONDS).floor() as u64;
        let flicker = 0.72 + 0.28 * Rng::new(self.sub_seed(4, 0, phase)).unit() as f32;
        if parts.lightning && now < self.bolt_end() {
            for hop in 0..self.knots.len() - 1 {
                self.hop_bolts(hop, now, phase, flicker, &mut frame.bolts);
            }
        }

        if parts.flashes {
            // Свечение вокруг летящей шашки.
            let arrival = self.arrival();
            let glow = if now < arrival {
                1.0
            } else {
                1.0 - (now - arrival) / HEAD_GLOW_FADE_SECONDS
            };
            if glow > 0.0 {
                frame.halo = Some(Flash {
                    center: self.head(now),
                    radius: HALO_RADIUS,
                    intensity: glow as f32 * flicker,
                });
            }
            // Вспышки на побитых шашках в момент удара.
            for strike in &self.strikes {
                let age = now - strike.time;
                if (0.0..FLASH_SECONDS).contains(&age) {
                    let u = (age / FLASH_SECONDS) as f32;
                    frame.flashes.push(Flash {
                        center: strike.center,
                        radius: 0.28 + 0.20 * u,
                        intensity: (1.0 - u) * (1.0 - u),
                    });
                }
            }
        }

        if parts.sparks {
            for (index, strike) in self.strikes.iter().enumerate() {
                if now < strike.time {
                    continue;
                }
                for number in 0..SPARKS_PER_PIECE {
                    let spark = self.spark_seed(index, number);
                    let age = (now - strike.time) as f32 - spark.delay;
                    if age < 0.0 || age >= spark.life {
                        continue;
                    }
                    let left = 1.0 - age / spark.life;
                    frame.sparks.push(Spark {
                        head: spark_position(spark.origin, spark.velocity, age),
                        tail: spark_position(
                            spark.origin,
                            spark.velocity,
                            (age - SPARK_TRAIL_SECONDS).max(0.0),
                        ),
                        intensity: left.powf(0.8),
                        heat: left,
                        size: spark.size,
                    });
                }
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

    /// Положение точки вдоль отрезка `a`–`b`: 0 в начале, 1 в конце.
    fn along(p: Point, a: Point, b: Point) -> f32 {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        ((p.0 - a.0) * dx + (p.1 - a.1) * dy) / (dx * dx + dy * dy)
    }

    const HOP: f64 = 0.21;
    const FIRST: Point = (0.5, 7.5);
    const LAST: Point = (6.5, 1.5);

    /// Шашка летит по диагонали через три побитые шашки: a1 → c3 → e5 → g7.
    fn sample() -> Discharge {
        let knots = vec![
            (FIRST, 10.0),
            ((2.5, 5.5), 10.0 + HOP),
            ((4.5, 3.5), 10.0 + 2.0 * HOP),
            (LAST, 10.0 + 3.0 * HOP),
        ];
        let direction = (0.707, -0.707);
        let strikes = vec![
            Strike {
                center: (1.5, 6.5),
                time: 10.0 + 0.5 * HOP,
                direction,
            },
            Strike {
                center: (3.5, 4.5),
                time: 10.0 + 1.5 * HOP,
                direction,
            },
            Strike {
                center: (5.5, 2.5),
                time: 10.0 + 2.5 * HOP,
                direction,
            },
        ];
        Discharge::new(knots, strikes, 42)
    }

    fn mains(frame: &DischargeFrame) -> Vec<&Bolt> {
        frame.bolts.iter().filter(|b| b.width == 1.0).collect()
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
                for (i, &p) in path.iter().enumerate() {
                    assert!(p.0.is_finite() && p.1.is_finite());
                    assert!(
                        deviation(p, a, b) <= limit,
                        "зерно {seed}: точка {p:?} ушла далеко"
                    );
                    // Вершины идут вдоль отрезка равномерно.
                    let expected = i as f32 / (path.len() - 1) as f32;
                    assert!(
                        (along(p, a, b) - expected).abs() < 1e-3,
                        "зерно {seed}: вершина {i}"
                    );
                }
                // Звенья мелкие, а сама молния изломана, а не прямая. Короткое боковое
                // ответвление из двух-четырёх звеньев может случайно выйти почти прямым,
                // поэтому излом проверяется у молний не короче клетки.
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
    fn easing_and_its_inverse_agree() {
        assert_eq!(ease(0.0), 0.0);
        assert_eq!(ease(1.0), 1.0);
        assert!((ease(0.5) - 0.5).abs() < 1e-12);
        for step in 0..=100 {
            let x = f64::from(step) / 100.0;
            assert!((ease_inverse(ease(x)) - x).abs() < 1e-6, "x = {x}");
            assert!((ease(ease_inverse(x)) - x).abs() < 1e-9, "u = {x}");
        }
        assert_eq!(ease(-3.0), 0.0);
        assert_eq!(ease(7.0), 1.0);
        assert!(ease_inverse(-1.0).abs() < 1e-9 && (ease_inverse(2.0) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn trail_is_bright_right_behind_the_piece_and_fades_out() {
        assert_eq!(trail_intensity(-0.01), 0.0);
        assert_eq!(trail_intensity(0.0), 1.0);
        assert_eq!(trail_intensity(TRAIL_FULL_SECONDS), 1.0);
        assert_eq!(trail_intensity(TRAIL_SECONDS), 0.0);
        let mut previous = 1.0;
        for step in 0..=100 {
            let value = trail_intensity(TRAIL_SECONDS * f64::from(step) / 100.0);
            assert!((0.0..=1.0).contains(&value) && value <= previous + 1e-6);
            previous = value;
        }
    }

    #[test]
    fn lightning_follows_the_path_of_the_piece_while_it_moves() {
        let fx = sample();
        let (start, a, b) = (10.0, FIRST, (2.5, 5.5));
        // До начала движения ничего нет.
        assert!(fx.frame(start - 0.01, EffectSet::ALL).is_empty());
        assert_eq!(fx.head(start - 1.0), a);
        assert_eq!(fx.start(), start);
        assert!((fx.arrival() - (start + 3.0 * HOP)).abs() < 1e-12);

        // Шашка летит по первому отрезку: молния идёт от начала пути до самой шашки.
        for step in 1..20 {
            let now = start + HOP * f64::from(step) / 20.0;
            let head = fx.head(now);
            assert!(
                deviation(head, a, b) < 1e-4,
                "шашка движется по прямой между полями"
            );
            let frame = fx.frame(now, EffectSet::ALL);
            let main = mains(&frame);
            assert!(
                !main.is_empty(),
                "шаг {step}: молния есть с самого начала движения"
            );
            let limit = max_bolt_deviation(distance(a, b)) + 1e-3;
            for bolt in &main {
                for &p in &bolt.points {
                    assert!(deviation(p, a, b) <= limit, "молния лежит на пути шашки");
                    assert!(
                        along(p, a, b) <= along(head, a, b) + 1e-3,
                        "молния не обгоняет шашку"
                    );
                }
            }
            // Участки идут подряд: от начала пути до шашки без разрывов.
            assert_eq!(main[0].points[0], a);
            for pair in main.windows(2) {
                assert_eq!(pair[0].points[pair[0].points.len() - 1], pair[1].points[0]);
            }
            let tail_end = *main[main.len() - 1].points.last().unwrap();
            assert!(
                distance(tail_end, head) < 1e-4,
                "шаг {step}: молния кончается у шашки"
            );
        }

        // На втором отрезке молния тянется уже через оба и по-прежнему кончается у шашки.
        let now = start + 1.5 * HOP;
        let frame = fx.frame(now, EffectSet::ALL);
        let reach = mains(&frame)
            .iter()
            .flat_map(|b| b.points.iter())
            .map(|&p| along(p, FIRST, LAST))
            .fold(0.0, f32::max);
        assert!((reach - along(fx.head(now), FIRST, LAST)).abs() < 1e-3);
        assert!(reach > 0.4 && reach < 0.6, "{reach}");
    }

    #[test]
    fn lightning_fades_behind_the_piece_and_goes_out_after_arrival() {
        let fx = sample();
        // В конце третьего отрезка начало пути уже погасло, а у шашки молния яркая.
        let now = fx.arrival() - 0.005;
        let frame = fx.frame(now, EffectSet::ALL);
        let main = mains(&frame);
        let tail = main
            .iter()
            .map(|b| along(b.points[0], FIRST, LAST))
            .fold(1.0, f32::min);
        assert!(tail > 0.15, "хвост молнии погас: она начинается с {tail}");
        let brightest = main.iter().map(|b| b.intensity).fold(0.0, f32::max);
        let dimmest = main.iter().map(|b| b.intensity).fold(1.0, f32::min);
        assert!(brightest > 0.7 && dimmest < 0.5, "{brightest} и {dimmest}");
        // Яркость растёт от хвоста к шашке.
        for pair in main.windows(2) {
            assert!(pair[0].intensity <= pair[1].intensity + 1e-4);
        }
        // Форма молнии меняется от такта к такту (мерцание), но внутри такта постоянна.
        let shapes = |t: f64| -> Vec<Vec<Point>> {
            let frame = fx.frame(t, EffectSet::ALL);
            frame
                .bolts
                .into_iter()
                .filter(|b| b.width == 1.0)
                .map(|b| b.points)
                .collect()
        };
        // Начало шестнадцатого такта: шашка уже стоит (она приходит через 0,63 с).
        let tick = 10.0 + 16.0 * FLICKER_SECONDS;
        assert!(tick > fx.arrival());
        assert_eq!(
            shapes(tick + 0.1 * FLICKER_SECONDS),
            shapes(tick + 0.4 * FLICKER_SECONDS)
        );
        assert_ne!(
            shapes(tick + 0.1 * FLICKER_SECONDS),
            shapes(tick + 1.1 * FLICKER_SECONDS)
        );
        // После остановки шашки молния гаснет, искры ещё летят, потом эффект заканчивается.
        assert!((fx.bolt_end() - (fx.arrival() + TRAIL_SECONDS)).abs() < 1e-12);
        assert!(!fx.frame(fx.bolt_end() - 0.1, EffectSet::ALL).bolts.is_empty());
        let after = fx.frame(fx.bolt_end() + 0.001, EffectSet::ALL);
        assert!(after.bolts.is_empty() && !after.sparks.is_empty());
        assert!(fx.bolt_end() < fx.end());
        assert!(!fx.is_finished(fx.end() - 0.01));
        assert!(fx.is_finished(fx.end()));
        assert!(fx.frame(fx.end(), EffectSet::ALL).is_empty());
    }

    #[test]
    fn pieces_are_struck_when_the_piece_flies_over_them() {
        let fx = sample();
        assert_eq!(fx.strikes().len(), 3);
        let near = |frame: &DischargeFrame, center: Point| {
            frame
                .sparks
                .iter()
                .filter(|s| distance(s.head, center) < 0.9)
                .count()
        };
        let flashes_at = |frame: &DischargeFrame, center: Point| {
            frame.flashes.iter().filter(|f| f.center == center).count()
        };
        for (index, strike) in fx.strikes().iter().enumerate() {
            // До пролёта — ни вспышки, ни искр у этой шашки.
            let before = fx.frame(strike.time - 0.005, EffectSet::ALL);
            assert_eq!(flashes_at(&before, strike.center), 0, "удар {index}");
            assert_eq!(near(&before, strike.center), 0, "удар {index}");
            // В момент пролёта шашка находится над побитой.
            assert!(
                distance(fx.head(strike.time), strike.center) < 1e-3,
                "удар {index}"
            );
            // Сразу после — вспышка и искры.
            let after = fx.frame(strike.time + 0.05, EffectSet::ALL);
            assert_eq!(flashes_at(&after, strike.center), 1, "удар {index}");
            assert!(
                near(&after, strike.center) >= SPARKS_PER_PIECE / 2,
                "удар {index}"
            );
        }
        // Искры разлетаются во все стороны, а не в одну.
        let last = fx.strikes()[2];
        let frame = fx.frame(last.time + 0.06, EffectSet::ALL);
        let (mut left, mut right, mut up, mut down) = (0, 0, 0, 0);
        for spark in frame
            .sparks
            .iter()
            .filter(|s| distance(s.head, last.center) < 0.9)
        {
            left += usize::from(spark.head.0 < last.center.0);
            right += usize::from(spark.head.0 > last.center.0);
            up += usize::from(spark.head.1 < last.center.1);
            down += usize::from(spark.head.1 > last.center.1);
        }
        assert!(
            left >= 4 && right >= 4 && up >= 4 && down >= 4,
            "{left} {right} {up} {down}"
        );
        // Яркости в допустимых пределах, искры не улетают далеко, след короткий.
        for step in 0..220 {
            let frame = fx.frame(10.0 + f64::from(step) * 0.008, EffectSet::ALL);
            for bolt in &frame.bolts {
                assert!((0.0..=1.0).contains(&bolt.intensity) && bolt.points.len() >= 2);
            }
            for flash in &frame.flashes {
                assert!((0.0..=1.0).contains(&flash.intensity) && flash.radius > 0.0 && flash.radius < 1.0);
            }
            for spark in &frame.sparks {
                assert!((0.0..=1.0).contains(&spark.intensity) && (0.0..=1.0).contains(&spark.heat));
                let nearest = fx
                    .strikes()
                    .iter()
                    .map(|s| distance(spark.head, s.center))
                    .fold(f32::MAX, f32::min);
                assert!(nearest < 3.5, "искра улетела на {nearest} клеток");
                assert!(distance(spark.head, spark.tail) < 0.45, "след искры короткий");
            }
        }
    }

    #[test]
    fn piece_glows_while_it_flies() {
        let fx = sample();
        let glow = |now: f64| fx.frame(now, EffectSet::ALL).halo;
        // Ореол держится на шашке всё время полёта и выступает из-под неё.
        for step in 0..=30 {
            let now = 10.0 + 3.0 * HOP * f64::from(step) / 30.0 - 1e-6;
            let halo = glow(now.max(10.0)).expect("ореол есть во время полёта");
            assert_eq!(halo.center, fx.head(now.max(10.0)));
            assert!(halo.radius > 0.40 && halo.radius < 1.0);
            assert!(halo.intensity > 0.5 && halo.intensity <= 1.0);
        }
        // После остановки он быстро гаснет.
        let fading = glow(fx.arrival() + HEAD_GLOW_FADE_SECONDS * 0.75).unwrap();
        assert!(fading.intensity < 0.3, "{}", fading.intensity);
        assert!(glow(fx.arrival() + HEAD_GLOW_FADE_SECONDS + 0.01).is_none());
        // Без вспышек и свечения ореола нет.
        let plain = EffectSet {
            flashes: false,
            ..EffectSet::ALL
        };
        assert!(fx.frame(10.1, plain).halo.is_none());
    }

    #[test]
    fn parts_of_the_effect_can_be_switched_independently() {
        let fx = sample();
        let now = 10.0 + 1.6 * HOP;
        let all = fx.frame(now, EffectSet::ALL);
        // Момент сразу после удара по второй шашке: на кадре есть всё.
        assert!(!all.bolts.is_empty() && !all.flashes.is_empty() && !all.sparks.is_empty());
        assert!(fx.frame(now, EffectSet::NONE).is_empty());
        assert!(EffectSet::ALL.any() && !EffectSet::NONE.any());

        let only_lightning = fx.frame(
            now,
            EffectSet {
                lightning: true,
                ..EffectSet::NONE
            },
        );
        assert_eq!(only_lightning.bolts, all.bolts);
        assert!(only_lightning.flashes.is_empty() && only_lightning.sparks.is_empty());
        assert!(only_lightning.halo.is_none());

        let only_sparks = fx.frame(
            now,
            EffectSet {
                sparks: true,
                ..EffectSet::NONE
            },
        );
        assert_eq!(only_sparks.sparks, all.sparks);
        assert!(only_sparks.bolts.is_empty() && only_sparks.flashes.is_empty());
        assert!(only_sparks.halo.is_none());

        let only_flashes = fx.frame(
            now,
            EffectSet {
                flashes: true,
                ..EffectSet::NONE
            },
        );
        assert_eq!(only_flashes.flashes, all.flashes);
        assert_eq!(only_flashes.halo, all.halo);
        assert!(all.halo.is_some());
        assert!(only_flashes.bolts.is_empty() && only_flashes.sparks.is_empty());

        let without_sparks = fx.frame(
            now,
            EffectSet {
                sparks: false,
                ..EffectSet::ALL
            },
        );
        assert!(without_sparks.sparks.is_empty());
        assert_eq!(without_sparks.bolts, all.bolts);
        assert!(EffectSet {
            sparks: true,
            ..EffectSet::NONE
        }
        .any());
    }

    #[test]
    fn hops_already_shown_flash_at_once() {
        // Первые два отрезка игрок прошёл по шагам: их моменты совпадают с началом.
        // Молния вспыхивает на них сразу, а дальше тянется за шашкой.
        let knots = vec![
            (FIRST, 5.0),
            ((2.5, 5.5), 5.0),
            ((4.5, 3.5), 5.0),
            (LAST, 5.0 + HOP),
        ];
        let direction = (0.707, -0.707);
        let strikes = vec![
            Strike {
                center: (1.5, 6.5),
                time: 5.0,
                direction,
            },
            Strike {
                center: (3.5, 4.5),
                time: 5.0,
                direction,
            },
            Strike {
                center: (5.5, 2.5),
                time: 5.0 + 0.5 * HOP,
                direction,
            },
        ];
        let fx = Discharge::new(knots, strikes, 9);
        assert_eq!(fx.head(5.0), (4.5, 3.5), "шашка уже стоит на третьем поле");
        assert_eq!(fx.head(4.0), (4.5, 3.5));
        let frame = fx.frame(5.0 + 0.01, EffectSet::ALL);
        let reach: Vec<f32> = mains(&frame)
            .iter()
            .flat_map(|b| b.points.iter())
            .map(|&p| along(p, FIRST, LAST))
            .collect();
        let low = reach.iter().copied().fold(1.0, f32::min);
        let high = reach.iter().copied().fold(0.0, f32::max);
        assert!(low < 0.01, "молния начинается от исходного поля: {low}");
        assert!(high > 0.66 && high < 0.75, "и доходит до шашки: {high}");
        // Две первые шашки поражены сразу, третья — при пролёте.
        let struck = |frame: &DischargeFrame, center: Point| frame.flashes.iter().any(|f| f.center == center);
        assert!(struck(&frame, (1.5, 6.5)) && struck(&frame, (3.5, 4.5)));
        assert!(!struck(&frame, (5.5, 2.5)));
        assert!(struck(&fx.frame(5.0 + 0.6 * HOP, EffectSet::ALL), (5.5, 2.5)));
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
        let mut c = sample();
        c.seed = 43;
        for step in 0..60 {
            let t = 10.0 + f64::from(step) * 0.03;
            assert_eq!(a.frame(t, EffectSet::ALL), b.frame(t, EffectSet::ALL));
        }
        assert_ne!(a.frame(10.1, EffectSet::ALL), c.frame(10.1, EffectSet::ALL));
        // Пустой и вырожденный пути не вызывают сбоя.
        let empty = Discharge::new(Vec::new(), Vec::new(), 1);
        assert!(empty.is_finished(0.0) && empty.frame(0.1, EffectSet::ALL).is_empty());
        assert_eq!(empty.head(1.0), (0.0, 0.0));
        let single = Discharge::new(vec![((1.0, 1.0), 0.0)], Vec::new(), 1);
        assert!(single.is_finished(0.0) && single.frame(0.0, EffectSet::ALL).is_empty());
        let still = Discharge::new(
            vec![((1.0, 1.0), 0.0), ((1.0, 1.0), 0.0), ((1.0, 1.0), 0.1)],
            vec![Strike {
                center: (1.0, 1.0),
                time: 0.0,
                direction: (0.0, 0.0),
            }],
            1,
        );
        for step in 0..80 {
            let frame = still.frame(f64::from(step) * 0.02, EffectSet::ALL);
            for bolt in &frame.bolts {
                assert!(bolt.points.iter().all(|p| p.0.is_finite() && p.1.is_finite()));
            }
            assert!(frame
                .sparks
                .iter()
                .all(|s| s.head.0.is_finite() && s.head.1.is_finite()));
        }
    }
}
