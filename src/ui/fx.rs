//! The electric discharge of a multi-capture: lightning, flashes and sparks.
//!
//! The lightning trails the capturing piece along its route and appears together with
//! the movement: the piece flies, the discharge follows it and gradually dies out
//! behind. Flying over a captured piece, the discharge strikes it: the piece flashes,
//! bursts into sparks and disappears.
//!
//! The module only computes a frame: the polylines of the lightning, the flashes and
//! the positions of the sparks. It knows nothing about the window and keeps no state
//! between frames: a frame is fully determined by the time and the seed, so the effect
//! is reproducible and covered by tests. The graphical part puts it on the screen with
//! the same hardware-accelerated drawing as the whole board.
//!
//! Coordinates are screen coordinates in cells: the unit is the side of a cell, the Y
//! axis points down, the origin is the top left corner of the playing field.

use crate::rng::Rng;
use std::f32::consts::TAU;

/// Smallest number of pieces captured in a move for the discharge to be shown.
pub const MIN_CAPTURES: u32 = 2;
/// How many seconds after the piece has passed the lightning on that part of the route goes out.
pub const TRAIL_SECONDS: f64 = 0.45;
/// How many seconds after the piece has passed a part of the lightning glows at full strength.
pub const TRAIL_FULL_SECONDS: f64 = 0.10;
/// Time a piece takes to disappear after being struck, s.
pub const VANISH_SECONDS: f64 = 0.12;
/// Length of the finest link of the lightning polyline, cells (approximately).
pub const BOLT_FINE_SEGMENT: f32 = 0.16;
/// Swing of the finest teeth, as a fraction of the length of a fine link.
pub const BOLT_FINE_BEND: f32 = 0.34;
/// How many times the swing of the teeth is smaller than that of the bends of the previous,
/// coarser level.
pub const BOLT_ROUGHNESS: f32 = 0.62;
/// Limit on the swing of the coarse bends, cells: the lightning does not stray far from the route.
pub const BOLT_MAX_BEND: f32 = 0.16;
/// Period at which the lightning changes its shape (flicker), s.
pub const FLICKER_SECONDS: f64 = 0.045;
/// Duration of a flash at the place of a strike, s.
pub const FLASH_SECONDS: f64 = 0.18;
/// Time the glow around the piece takes to go out after the piece has stopped, s.
pub const HEAD_GLOW_FADE_SECONDS: f64 = 0.12;
/// Outer radius of the glow around the flying piece, cells (the piece itself is 0.40).
pub const HALO_RADIUS: f32 = 0.66;
/// Number of sparks from one captured piece.
pub const SPARKS_PER_PIECE: usize = 28;
/// Lifetime of a spark, s: from and to.
pub const SPARK_LIFE: (f32, f32) = (0.35, 0.90);
/// Initial speed of a spark, cells per second: from and to.
pub const SPARK_SPEED: (f32, f32) = (1.4, 5.6);
/// Addition to the speed of sparks in the direction the piece is flying, cells per second.
pub const SPARK_CARRY: f32 = 1.3;
/// Air drag: the speed of a spark falls e times in `1 / SPARK_DRAG` seconds.
pub const SPARK_DRAG: f32 = 3.2;
/// Acceleration at which sparks fall down the screen, cells/s².
pub const SPARK_GRAVITY: f32 = 5.5;
/// Length of the glowing trail of a spark in time, s.
pub const SPARK_TRAIL_SECONDS: f32 = 0.045;

/// Colour of the core of the lightning: almost white with a greenish-yellow tint.
pub const BOLT_CORE: [f32; 3] = [0.93, 1.00, 0.70];
/// Colour of the glow of the lightning and sparks: greenish yellow.
pub const GLOW: [f32; 3] = [0.66, 0.95, 0.08];

/// A point in screen coordinates of the board (in cells).
pub type Point = (f32, f32);

/// Which parts of the effect are shown.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct EffectSet {
    /// Lightning behind the piece.
    pub lightning: bool,
    /// Sparks from captured pieces.
    pub sparks: bool,
    /// Flashes at the places of strikes and the glow around the flying piece.
    pub flashes: bool,
}

impl EffectSet {
    /// All parts are on.
    pub const ALL: Self = Self {
        lightning: true,
        sparks: true,
        flashes: true,
    };
    /// All parts are off.
    pub const NONE: Self = Self {
        lightning: false,
        sparks: false,
        flashes: false,
    };

    /// Whether at least one part is on.
    #[must_use]
    pub const fn any(self) -> bool {
        self.lightning || self.sparks || self.flashes
    }
}

/// Smooth acceleration and braking of the piece on a leg of the route: the fraction of
/// the distance covered for a fraction of the time elapsed.
#[must_use]
pub fn ease(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// The inverse of [`ease`]: the fraction of the time in which the fraction `u` of the
/// distance is covered.
#[must_use]
pub fn ease_inverse(u: f64) -> f64 {
    let u = u.clamp(0.0, 1.0);
    0.5 - ((1.0 - 2.0 * u).asin() / 3.0).sin()
}

/// Brightness of a part of the lightning `age` seconds after the piece has flown over it.
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

/// Colour of a spark by its "temperature": 1 is a spark that has just flown out (almost
/// white), 0 is one burning out (dark yellow). In mid-life a spark is greenish yellow.
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

/// Position of a spark `age` seconds after it left the point `origin` at the velocity
/// `velocity` (cells/s).
///
/// Air drag and gravity act on a spark: `dv/dt = −k·v + g`. The exact solution of this
/// equation is used, so a frame does not depend on the refresh rate of the screen.
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

/// Velocity of a spark `age` seconds after it flew out.
#[must_use]
pub fn spark_velocity(velocity: Point, age: f32) -> Point {
    let fade = (-SPARK_DRAG * age.max(0.0)).exp();
    (
        velocity.0 * fade,
        velocity.1 * fade + SPARK_GRAVITY / SPARK_DRAG * (1.0 - fade),
    )
}

/// Swings of the midpoint displacement for a lightning of the length `length`: from
/// the coarsest bend to the finest teeth. The number of levels is chosen so that a fine
/// link is about [`BOLT_FINE_SEGMENT`] long, so short and long bolts are jagged equally
/// finely.
fn bolt_amplitudes(length: f32) -> Vec<f32> {
    let levels = ((length / BOLT_FINE_SEGMENT).max(1.0).log2().ceil() as usize).clamp(1, 7);
    let fine = BOLT_FINE_BEND * length / (1u32 << levels) as f32;
    (0..levels)
        .map(|level| (fine / BOLT_ROUGHNESS.powi((levels - 1 - level) as i32)).min(BOLT_MAX_BEND))
        .collect()
}

/// The lightning polyline from `a` to `b`: a segment broken by midpoint displacement.
///
/// Each pass halves every link and shifts the midpoints across the overall direction;
/// the swing of the shift decreases from pass to pass. The ends stay in place, and the
/// vertices are spread evenly along the segment. The same seed gives the same polyline.
#[must_use]
pub fn bolt_path(a: Point, b: Point, seed: u64) -> Vec<Point> {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length = dx.hypot(dy);
    if !length.is_finite() || length < 1e-4 {
        return vec![a, b];
    }
    // Unit normal to the direction of the lightning.
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

/// The greatest deviation of the polyline points from the straight line that
/// [`bolt_path`] can produce for a segment of the length `length`.
#[must_use]
pub fn max_bolt_deviation(length: f32) -> f32 {
    if !length.is_finite() || length < 1e-4 {
        return 0.0;
    }
    bolt_amplitudes(length).iter().sum()
}

/// A part of the lightning in a frame.
#[derive(Clone, PartialEq, Debug)]
pub struct Bolt {
    /// Vertices of the polyline.
    pub points: Vec<Point>,
    /// Brightness 0…1.
    pub intensity: f32,
    /// Relative thickness: 1 is the main channel, less is a side branch.
    pub width: f32,
}

/// A flash or a glow.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Flash {
    pub center: Point,
    /// Radius in cells.
    pub radius: f32,
    /// Brightness 0…1.
    pub intensity: f32,
}

/// A spark in a frame: a short glowing stroke from the tail to the head.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Spark {
    pub head: Point,
    pub tail: Point,
    /// Brightness 0…1.
    pub intensity: f32,
    /// "Temperature" 1…0, see [`spark_color`].
    pub heat: f32,
    /// Relative thickness of the stroke.
    pub size: f32,
}

/// A frame of the discharge.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct DischargeFrame {
    /// Lightning behind the piece.
    pub bolts: Vec<Bolt>,
    /// Glow around the flying piece: a halo that shows from under it.
    pub halo: Option<Flash>,
    /// Flashes on the captured pieces.
    pub flashes: Vec<Flash>,
    pub sparks: Vec<Spark>,
}

impl DischargeFrame {
    /// Whether there is nothing to draw.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bolts.is_empty() && self.halo.is_none() && self.flashes.is_empty() && self.sparks.is_empty()
    }
}

/// A strike of the discharge on a captured piece.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Strike {
    /// Where the captured piece stands.
    pub center: Point,
    /// The moment of the strike: the capturing piece flies over the captured one.
    pub time: f64,
    /// Unit vector of the direction the capturing piece is flying in at that moment.
    pub direction: Point,
}

/// Parameters of one spark, computed from the seed.
struct SparkSeed {
    origin: Point,
    velocity: Point,
    delay: f32,
    life: f32,
    size: f32,
}

/// Number of polyline links in one part of the lightning with a common brightness.
const CHUNK: usize = 4;

/// The discharge that comes with one move.
#[derive(Clone, PartialEq, Debug)]
pub struct Discharge {
    /// The squares of the route of the capturing piece and the moments it is on them.
    knots: Vec<(Point, f64)>,
    strikes: Vec<Strike>,
    seed: u64,
}

impl Discharge {
    /// A discharge along the route `knots` (points and moments, in order) with the
    /// strikes `strikes`. `seed` determines the shape of the lightning and the scatter of sparks.
    #[must_use]
    pub const fn new(knots: Vec<(Point, f64)>, strikes: Vec<Strike>, seed: u64) -> Self {
        Self { knots, strikes, seed }
    }

    /// The moment it starts: the piece sets off.
    #[must_use]
    pub fn start(&self) -> f64 {
        self.knots.first().map_or(0.0, |k| k.1)
    }

    /// The moment the piece lands on its final square.
    #[must_use]
    pub fn arrival(&self) -> f64 {
        self.knots.last().map_or(0.0, |k| k.1)
    }

    /// The strikes on the captured pieces.
    #[must_use]
    pub fn strikes(&self) -> &[Strike] {
        &self.strikes
    }

    /// The moment the lightning goes out.
    #[must_use]
    pub fn bolt_end(&self) -> f64 {
        self.arrival() + TRAIL_SECONDS
    }

    /// The moment the last spark goes out and the effect ends.
    pub fn end(&self) -> f64 {
        let last_strike = self.strikes.iter().map(|s| s.time).fold(self.start(), f64::max);
        (last_strike + f64::from(SPARK_LIFE.1) + 0.05).max(self.bolt_end())
    }

    /// Whether the effect is over by the moment `now`.
    #[must_use]
    pub fn is_finished(&self, now: f64) -> bool {
        self.knots.len() < 2 || now >= self.end()
    }

    /// Position of the capturing piece at the moment `now`.
    #[must_use]
    pub fn head(&self, now: f64) -> Point {
        let Some(&(first, start)) = self.knots.first() else {
            return (0.0, 0.0);
        };
        if now <= start {
            // Legs passed in no time (jumps already shown) are skipped.
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

    /// A seed for a part of the effect: it does not depend on the order of computation.
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
            // Sparks fly out not from one point but from a small circle inside the piece.
            origin: (hit.center.0 + cos * 0.10, hit.center.1 + sin * 0.10),
            // They scatter in all directions, drifting slightly along the piece's flight
            // and upwards.
            velocity: (
                cos * speed + hit.direction.0 * SPARK_CARRY,
                sin * speed + hit.direction.1 * SPARK_CARRY - 0.9,
            ),
            delay: rng.unit() as f32 * 0.035,
            life: SPARK_LIFE.0 + (SPARK_LIFE.1 - SPARK_LIFE.0) * rng.unit() as f32,
            size: 0.55 + 0.45 * rng.unit() as f32,
        }
    }

    /// The lightning on the leg number `hop` of the route: from its start to the piece (or
    /// to the end of the leg if the piece has flown on), split into parts by age.
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
            // The lightning ends exactly at the piece.
            let u = reached as f32;
            visible.push((a.0 + (b.0 - a.0) * u, a.1 + (b.1 - a.1) * u));
        }
        if visible.len() < 2 {
            return;
        }
        // When the piece flew past the point at the fraction `along` of the leg.
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

        // A side branch near the freshest part.
        if newest > 0.25 && visible.len() > 3 {
            let mut rng = Rng::new(self.sub_seed(3, hop as u64, phase));
            let vertex = visible[visible.len() - 2 - rng.below((visible.len() - 2).min(4))];
            let along = (b.1 - a.1).atan2(b.0 - a.0);
            // The branch goes backwards and sideways from the direction of flight.
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

    /// The frame of the effect at the moment `now`; only the parts in `parts` are shown.
    #[must_use]
    pub fn frame(&self, now: f64, parts: EffectSet) -> DischargeFrame {
        let mut frame = DischargeFrame::default();
        if self.knots.len() < 2 || now < self.start() || now >= self.end() {
            return frame;
        }

        // The lightning: it trails the piece, flickers and dies out behind it.
        let phase = ((now - self.start()) / FLICKER_SECONDS).floor() as u64;
        let flicker = 0.72 + 0.28 * Rng::new(self.sub_seed(4, 0, phase)).unit() as f32;
        if parts.lightning && now < self.bolt_end() {
            for hop in 0..self.knots.len() - 1 {
                self.hop_bolts(hop, now, phase, flicker, &mut frame.bolts);
            }
        }

        if parts.flashes {
            // The glow around the flying piece.
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
            // Flashes on the captured pieces at the moment of the strike.
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

    /// Distance from a point to the straight line through `a` and `b`.
    fn deviation(p: Point, a: Point, b: Point) -> f32 {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        ((p.0 - a.0) * dy - (p.1 - a.1) * dx).abs() / dx.hypot(dy)
    }

    /// Position of a point along the segment `a`–`b`: 0 at the start, 1 at the end.
    fn along(p: Point, a: Point, b: Point) -> f32 {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        ((p.0 - a.0) * dx + (p.1 - a.1) * dy) / (dx * dx + dy * dy)
    }

    const HOP: f64 = 0.21;
    const FIRST: Point = (0.5, 7.5);
    const LAST: Point = (6.5, 1.5);

    /// The piece flies along a diagonal over three captured pieces: a1 → c3 → e5 → g7.
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
        // A short, a medium and a long bolt, one of them along the diagonal of the whole board.
        let ends = [
            ((1.5, 6.5), (2.5, 5.5)),
            ((1.5, 6.5), (4.5, 3.5)),
            ((0.5, 7.5), (7.5, 0.5)),
            ((6.5, 2.5), (6.9, 2.6)),
        ];
        for (a, b) in ends {
            let straight = distance(a, b);
            let limit = max_bolt_deviation(straight) + 1e-4;
            assert!(
                limit < 0.9,
                "the lightning strays less than a cell from the line: {limit}"
            );
            for seed in 0..200u64 {
                let path = bolt_path(a, b, seed);
                assert_eq!(path[0], a);
                assert_eq!(path[path.len() - 1], b);
                assert!((path.len() - 1).is_power_of_two());
                for (i, &p) in path.iter().enumerate() {
                    assert!(p.0.is_finite() && p.1.is_finite());
                    assert!(
                        deviation(p, a, b) <= limit,
                        "seed {seed}: the point {p:?} went far"
                    );
                    // The vertices are spread evenly along the segment.
                    let expected = i as f32 / (path.len() - 1) as f32;
                    assert!(
                        (along(p, a, b) - expected).abs() < 1e-3,
                        "seed {seed}: vertex {i}"
                    );
                }
                // The links are fine, and the bolt itself is jagged, not straight. A short
                // side branch of two to four links can come out almost straight by chance,
                // so jaggedness is checked for bolts at least a cell long.
                let length: f32 = path.windows(2).map(|w| distance(w[0], w[1])).sum();
                assert!(length >= straight);
                if straight >= 1.0 {
                    assert!(
                        length > straight * 1.015,
                        "seed {seed}: {} of {straight}",
                        length / straight
                    );
                }
                let longest = path.windows(2).map(|w| distance(w[0], w[1])).fold(0.0, f32::max);
                assert!(longest < 2.5 * BOLT_FINE_SEGMENT, "seed {seed}: link {longest}");
            }
        }
    }

    #[test]
    fn bolt_shape_depends_only_on_the_seed() {
        let (a, b) = ((0.5, 0.5), (3.5, 3.5));
        assert_eq!(bolt_path(a, b, 7), bolt_path(a, b, 7));
        assert_ne!(bolt_path(a, b, 7), bolt_path(a, b, 8));
        // The number of links grows with the length but is limited.
        assert_eq!(bolt_path(a, b, 1).len(), 33);
        assert_eq!(bolt_path((0.0, 0.0), (0.1, 0.0), 1).len(), 3);
        assert_eq!(bolt_path((0.0, 0.0), (1000.0, 0.0), 1).len(), 129);
        // Degenerate cases give neither a failure nor NaN.
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
        // Nothing before the movement starts.
        assert!(fx.frame(start - 0.01, EffectSet::ALL).is_empty());
        assert_eq!(fx.head(start - 1.0), a);
        assert_eq!(fx.start(), start);
        assert!((fx.arrival() - (start + 3.0 * HOP)).abs() < 1e-12);

        // The piece flies along the first leg: the lightning runs from the start of the
        // route to the piece.
        for step in 1..20 {
            let now = start + HOP * f64::from(step) / 20.0;
            let head = fx.head(now);
            assert!(
                deviation(head, a, b) < 1e-4,
                "the piece moves in a straight line between squares"
            );
            let frame = fx.frame(now, EffectSet::ALL);
            let main = mains(&frame);
            assert!(
                !main.is_empty(),
                "step {step}: the lightning is there from the very start of the movement"
            );
            let limit = max_bolt_deviation(distance(a, b)) + 1e-3;
            for bolt in &main {
                for &p in &bolt.points {
                    assert!(
                        deviation(p, a, b) <= limit,
                        "the lightning lies on the route of the piece"
                    );
                    assert!(
                        along(p, a, b) <= along(head, a, b) + 1e-3,
                        "the lightning does not overtake the piece"
                    );
                }
            }
            // The parts follow one another: from the start of the route to the piece without gaps.
            assert_eq!(main[0].points[0], a);
            for pair in main.windows(2) {
                assert_eq!(pair[0].points[pair[0].points.len() - 1], pair[1].points[0]);
            }
            let tail_end = *main[main.len() - 1].points.last().unwrap();
            assert!(
                distance(tail_end, head) < 1e-4,
                "step {step}: the lightning ends at the piece"
            );
        }

        // On the second leg the lightning runs through both and still ends at the piece.
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
        // At the end of the third leg the start of the route has gone out, and at the piece
        // the lightning is bright.
        let now = fx.arrival() - 0.005;
        let frame = fx.frame(now, EffectSet::ALL);
        let main = mains(&frame);
        let tail = main
            .iter()
            .map(|b| along(b.points[0], FIRST, LAST))
            .fold(1.0, f32::min);
        assert!(
            tail > 0.15,
            "the tail of the lightning has gone out: it starts at {tail}"
        );
        let brightest = main.iter().map(|b| b.intensity).fold(0.0, f32::max);
        let dimmest = main.iter().map(|b| b.intensity).fold(1.0, f32::min);
        assert!(brightest > 0.7 && dimmest < 0.5, "{brightest} and {dimmest}");
        // The brightness grows from the tail to the piece.
        for pair in main.windows(2) {
            assert!(pair[0].intensity <= pair[1].intensity + 1e-4);
        }
        // The shape of the lightning changes from tick to tick (flicker) but is constant
        // within a tick.
        let shapes = |t: f64| -> Vec<Vec<Point>> {
            let frame = fx.frame(t, EffectSet::ALL);
            frame
                .bolts
                .into_iter()
                .filter(|b| b.width == 1.0)
                .map(|b| b.points)
                .collect()
        };
        // The start of the sixteenth tick: the piece already stands (it arrives after 0.63 s).
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
        // After the piece stops the lightning goes out, sparks still fly, then the effect ends.
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
            // Before the fly-over: neither a flash nor sparks at this piece.
            let before = fx.frame(strike.time - 0.005, EffectSet::ALL);
            assert_eq!(flashes_at(&before, strike.center), 0, "strike {index}");
            assert_eq!(near(&before, strike.center), 0, "strike {index}");
            // At the moment of the fly-over the piece is above the captured one.
            assert!(
                distance(fx.head(strike.time), strike.center) < 1e-3,
                "strike {index}"
            );
            // Right after it: a flash and sparks.
            let after = fx.frame(strike.time + 0.05, EffectSet::ALL);
            assert_eq!(flashes_at(&after, strike.center), 1, "strike {index}");
            assert!(
                near(&after, strike.center) >= SPARKS_PER_PIECE / 2,
                "strike {index}"
            );
        }
        // The sparks scatter in all directions, not in one.
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
        // Brightness stays within bounds, sparks do not fly far, the trail is short.
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
                assert!(nearest < 3.5, "a spark flew {nearest} cells away");
                assert!(
                    distance(spark.head, spark.tail) < 0.45,
                    "the trail of a spark is short"
                );
            }
        }
    }

    #[test]
    fn piece_glows_while_it_flies() {
        let fx = sample();
        let glow = |now: f64| fx.frame(now, EffectSet::ALL).halo;
        // The halo stays on the piece for the whole flight and shows from under it.
        for step in 0..=30 {
            let now = 10.0 + 3.0 * HOP * f64::from(step) / 30.0 - 1e-6;
            let halo = glow(now.max(10.0)).expect("the halo is there during the flight");
            assert_eq!(halo.center, fx.head(now.max(10.0)));
            assert!(halo.radius > 0.40 && halo.radius < 1.0);
            assert!(halo.intensity > 0.5 && halo.intensity <= 1.0);
        }
        // After the stop it goes out quickly.
        let fading = glow(fx.arrival() + HEAD_GLOW_FADE_SECONDS * 0.75).unwrap();
        assert!(fading.intensity < 0.3, "{}", fading.intensity);
        assert!(glow(fx.arrival() + HEAD_GLOW_FADE_SECONDS + 0.01).is_none());
        // Without flashes and glow there is no halo.
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
        // The moment right after the strike on the second piece: the frame has everything.
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
        // The player made the first two legs step by step: their moments coincide with
        // the start. The lightning flashes on them at once and then trails the piece.
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
        assert_eq!(
            fx.head(5.0),
            (4.5, 3.5),
            "the piece already stands on the third square"
        );
        assert_eq!(fx.head(4.0), (4.5, 3.5));
        let frame = fx.frame(5.0 + 0.01, EffectSet::ALL);
        let reach: Vec<f32> = mains(&frame)
            .iter()
            .flat_map(|b| b.points.iter())
            .map(|&p| along(p, FIRST, LAST))
            .collect();
        let low = reach.iter().copied().fold(1.0, f32::min);
        let high = reach.iter().copied().fold(0.0, f32::max);
        assert!(low < 0.01, "the lightning starts from the starting square: {low}");
        assert!(high > 0.66 && high < 0.75, "and reaches the piece: {high}");
        // The first two pieces are struck at once, the third at the fly-over.
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
            "before flying out the spark is in place"
        );

        // The exact solution agrees with step-by-step integration of dv/dt = −k·v + g.
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
                assert!(distance(p, exact) < 2e-3, "t = {age}: {p:?} and {exact:?}");
                assert!(distance(v, exact_v) < 2e-3, "t = {age}: {v:?} and {exact_v:?}");
            }
        }
        // The horizontal speed dies out; the vertical one tends to the terminal falling speed.
        let late = spark_velocity(velocity, 5.0);
        assert!(late.0.abs() < 1e-3);
        assert!((late.1 - SPARK_GRAVITY / SPARK_DRAG).abs() < 1e-3);
        // A spark that flew upwards first rises, then falls below the point it left.
        let up = (0.0, -3.0);
        assert!(spark_position(origin, up, 0.15).1 < origin.1);
        assert!(spark_position(origin, up, 2.0).1 > origin.1);
    }

    #[test]
    fn colours_are_greenish_yellow() {
        /// Hue in degrees: 60 is yellow, 120 is green.
        fn hue(c: [f32; 3]) -> f32 {
            let max = c[0].max(c[1]).max(c[2]);
            let min = c[0].min(c[1]).min(c[2]);
            assert!(max - min > 0.05, "the colour is not grey: {c:?}");
            let h = if max == c[0] {
                (c[1] - c[2]) / (max - min)
            } else if max == c[1] {
                2.0 + (c[2] - c[0]) / (max - min)
            } else {
                4.0 + (c[0] - c[1]) / (max - min)
            };
            (60.0 * h).rem_euclid(360.0)
        }
        // The glow: yellow with a green tint, between yellow (60°) and yellow-green (90°).
        assert!((66.0..=85.0).contains(&hue(GLOW)), "{}", hue(GLOW));
        for step in 0..=20 {
            let heat = step as f32 / 20.0;
            let c = spark_color(heat);
            assert!(c.iter().all(|v| (0.0..=1.0).contains(v)));
            assert!(
                (64.0..=85.0).contains(&hue(c)),
                "temperature {heat}: hue {}",
                hue(c)
            );
            assert!(
                c[1] >= c[0] && c[0] > c[2],
                "the green component is not less than the red one: {c:?}"
            );
        }
        // A hot spark is lighter than a cooled one.
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
        // An empty and a degenerate route cause no failure.
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
