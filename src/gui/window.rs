//! Size and place of the main window: restoring them at the start and noticing when
//! the player changes them.
//!
//! The system is asked for the saved place when the window is created. Not every system
//! keeps to it: a screen may be gone, screens may differ in density, a window manager
//! may put the window where it likes. So the first moments after the start check where
//! the window has come up, move it to the saved place once, and centre it on the main
//! screen if the screen it stood on is no longer there. A window saved maximized is
//! maximized again once it is shown, if the system has not done it already.

use eframe::egui;
use rucheckers::settings::{WindowGeometry, WindowPlace};
use std::time::Duration;

/// How long the window keeps its size and place before they are saved, in seconds. A
/// window being dragged is not saved at every step.
const SETTLE: f64 = 0.7;
/// How long the window must stand still before the place it has come up in is judged,
/// in seconds: window managers place a window shortly after it appears.
const STILL: f64 = 0.25;
/// The longest wait at each step of placing the window, in seconds.
const PLACING_WAIT: f64 = 1.5;
/// The difference in desktop units that counts as another place or another screen.
const NEAR: f32 = 1.5;
/// The shortest wait between frames while the window is placed.
const MIN_WAKE: f64 = 0.01;

/// Whether the system places windows in points rather than in pixels.
const IN_POINTS: bool = cfg!(target_os = "macos");

/// What the window looks like in one frame, in desktop units: the units in which the
/// system places windows.
#[derive(Clone, Copy, PartialEq, Debug)]
struct View {
    /// Desktop units in one point of the interface.
    per_point: f32,
    /// Desktop units in one point at the usual zoom: the scale of the screen.
    scale: f32,
    /// Size of the inside of the window in points at the usual zoom.
    size: [f32; 2],
    /// Top left corner of the window frame. Unknown on Wayland.
    position: Option<[f32; 2]>,
    /// Size of the window frame.
    outer_size: Option<[f32; 2]>,
    /// Size of the screen the window stands on.
    screen: Option<[f32; 2]>,
    maximized: bool,
    /// The window is minimized or covers the whole screen: its size and place now are
    /// not those to keep.
    hidden: bool,
}

impl View {
    fn read(ctx: &egui::Context) -> Option<Self> {
        let zoom = ctx.zoom_factor();
        let size = ctx.screen_rect().size() * zoom;
        ctx.input(|input| Self::from_info(input.viewport(), zoom, [size.x, size.y]))
    }

    fn from_info(info: &egui::ViewportInfo, zoom: f32, size: [f32; 2]) -> Option<Self> {
        let native = info.native_pixels_per_point?;
        let scale = if IN_POINTS { 1.0 } else { native };
        let per_point = zoom * scale;
        let desktop = |v: egui::Vec2| [v.x * per_point, v.y * per_point];
        Some(Self {
            per_point,
            scale,
            size,
            position: info.outer_rect.map(|rect| desktop(rect.min.to_vec2())),
            outer_size: info.outer_rect.map(|rect| desktop(rect.size())),
            screen: info.monitor_size.map(desktop),
            // macOS reports the state only when the window is created.
            maximized: !IN_POINTS && info.maximized == Some(true),
            hidden: info.minimized == Some(true) || info.fullscreen == Some(true),
        })
    }
}

/// Size and place of the window in the units of the settings.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Look {
    maximized: bool,
    size: Option<[u32; 2]>,
    place: Option<WindowPlace>,
}

impl Look {
    fn of(view: &View) -> Self {
        let size = [view.size[0].round() as u32, view.size[1].round() as u32];
        let place = view
            .position
            .zip(view.screen)
            .map(|(position, screen)| WindowPlace {
                x: position[0].round() as i32,
                y: position[1].round() as i32,
                screen_width: screen[0].round() as u32,
                screen_height: screen[1].round() as u32,
                scale: (view.scale * 100.0).round() as u32,
            });
        Self {
            maximized: view.maximized,
            size: WindowGeometry::size_is_valid(size[0], size[1]).then_some(size),
            place: place.filter(WindowPlace::is_valid),
        }
    }
}

/// What has to be done after a frame.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
struct Step {
    /// Size and place to save.
    report: Option<WindowGeometry>,
    /// Where to move the window frame, in desktop units.
    move_to: Option<[f32; 2]>,
    /// Whether to maximize the window.
    maximize: bool,
    /// When to look at the window again, in seconds from now.
    wake: Option<f64>,
}

impl Step {
    fn wake(seconds: f64) -> Self {
        Self {
            wake: Some(seconds.max(MIN_WAKE)),
            ..Self::default()
        }
    }

    fn move_to(position: [f32; 2]) -> Self {
        Self {
            move_to: Some(position),
            // A moved window gets a new frame by itself; this is in case it does not.
            wake: Some(STILL),
            ..Self::default()
        }
    }

    fn maximize() -> Self {
        Self {
            maximize: true,
            wake: Some(STILL),
            ..Self::default()
        }
    }
}

/// The steps of placing the window after the start.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Stage {
    /// Waiting for the window to come up.
    Start,
    /// Waiting for a window saved maximized to come up. Some systems do not maximize a
    /// window before it is shown.
    Maximizing,
    /// The window is being moved to the saved place.
    Moving,
    /// The screen of the saved place is gone, and the window is being moved to the main
    /// screen.
    ToMainScreen,
    /// The window is in place; changes are saved.
    Tracking,
}

/// The place and screen of the window in one frame.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Spot {
    position: Option<[f32; 2]>,
    outer_size: Option<[f32; 2]>,
    screen: Option<[f32; 2]>,
}

impl Spot {
    const fn of(view: &View) -> Self {
        Self {
            position: view.position,
            outer_size: view.outer_size,
            screen: view.screen,
        }
    }
}

/// A size and place waiting to be saved.
#[derive(Clone, Copy, Debug)]
struct Pending {
    look: Look,
    /// When the window took them.
    since: f64,
    reported: bool,
}

/// Watches the main window.
pub struct Tracker {
    /// The size and place the window has when it is not maximized.
    normal: WindowGeometry,
    stage: Stage,
    /// When the current stage began; set at the first frame.
    stage_since: Option<f64>,
    /// Where the window stands and since when.
    still: Option<(Spot, f64)>,
    pending: Option<Pending>,
}

impl Tracker {
    /// A tracker for the window opened by [`restore`] with the saved geometry.
    pub fn new(saved: WindowGeometry) -> Self {
        // A maximized window covers its screen, wherever its normal place is.
        let stage = if saved.maximized && !IN_POINTS {
            Stage::Maximizing
        } else if saved.place.is_none() {
            Stage::Tracking
        } else {
            Stage::Start
        };
        Self {
            normal: WindowGeometry {
                maximized: false,
                ..saved
            },
            stage,
            stage_since: None,
            still: None,
            pending: None,
        }
    }

    /// Looks at the window after a frame. Returns the size and place to save when they
    /// have changed and settled.
    pub fn update(&mut self, ctx: &egui::Context, now: f64) -> Option<WindowGeometry> {
        let view = View::read(ctx)?;
        let step = self.step(&view, now);
        if let Some([x, y]) = step.move_to {
            let position = egui::pos2(x / view.per_point, y / view.per_point);
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(position));
        }
        if step.maximize {
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
        }
        if let Some(wake) = step.wake {
            ctx.request_repaint_after(Duration::from_secs_f64(wake));
        }
        step.report
    }

    /// The size and place not saved yet, when the window is closed.
    pub fn last(&mut self) -> Option<WindowGeometry> {
        let pending = self.pending.take()?;
        (self.stage == Stage::Tracking && !pending.reported).then(|| self.keep(pending.look))
    }

    fn step(&mut self, view: &View, now: f64) -> Step {
        if self.stage == Stage::Tracking {
            return self.track(view, now);
        }
        let stage_since = *self.stage_since.get_or_insert(now);
        let spot = Spot::of(view);
        let still_since = match self.still {
            Some((seen, since)) if seen == spot => since,
            _ => {
                self.still = Some((spot, now));
                now
            }
        };
        let still_left = STILL - (now - still_since);
        let wait_left = PLACING_WAIT - (now - stage_since);
        if still_left > 0.0 && wait_left > 0.0 {
            return Step::wake(still_left.min(wait_left));
        }

        let place = self.normal.place;
        match self.stage {
            Stage::Start => {
                let target = place.map(|p| [p.x as f32, p.y as f32]);
                match view.position.zip(target) {
                    Some((position, target)) if far(position, target) => {
                        self.enter(Stage::Moving, now);
                        Step::move_to(target)
                    }
                    Some(_) => self.check_screen(view, now),
                    None => self.enter_tracking(view, now),
                }
            }
            Stage::Maximizing if !view.maximized => {
                self.enter(Stage::Tracking, now);
                Step::maximize()
            }
            Stage::Maximizing => self.enter_tracking(view, now),
            Stage::Moving => self.check_screen(view, now),
            Stage::ToMainScreen => {
                let centre = view.screen.zip(view.outer_size).map(|(screen, outer)| {
                    [
                        ((screen[0] - outer[0]) / 2.0).max(0.0).round(),
                        ((screen[1] - outer[1]) / 2.0).max(0.0).round(),
                    ]
                });
                match centre {
                    Some(centre) => {
                        self.enter(Stage::Tracking, now);
                        Step::move_to(centre)
                    }
                    None => self.enter_tracking(view, now),
                }
            }
            Stage::Tracking => unreachable!("handled above"),
        }
    }

    /// Whether the window has come up on a screen of the size it was saved on. If not,
    /// it is moved to the main screen, whose corner is the origin of the desktop.
    fn check_screen(&mut self, view: &View, now: f64) -> Step {
        let saved = self
            .normal
            .place
            .map(|p| [p.screen_width as f32, p.screen_height as f32]);
        match view.screen.zip(saved) {
            Some((screen, saved)) if far(screen, saved) => {
                self.enter(Stage::ToMainScreen, now);
                Step::move_to([0.0, 0.0])
            }
            _ => self.enter_tracking(view, now),
        }
    }

    fn enter(&mut self, stage: Stage, now: f64) {
        self.stage = stage;
        self.stage_since = Some(now);
        self.still = None;
    }

    fn enter_tracking(&mut self, view: &View, now: f64) -> Step {
        self.enter(Stage::Tracking, now);
        self.track(view, now)
    }

    fn track(&mut self, view: &View, now: f64) -> Step {
        if view.hidden {
            self.pending = None;
            return Step::default();
        }
        let look = Look::of(view);
        let pending = match &mut self.pending {
            Some(pending) if pending.look == look => pending,
            _ => {
                self.pending = Some(Pending {
                    look,
                    since: now,
                    reported: false,
                });
                return Step::wake(SETTLE);
            }
        };
        if pending.reported {
            return Step::default();
        }
        let left = SETTLE - (now - pending.since);
        if left > 0.0 {
            return Step::wake(left);
        }
        pending.reported = true;
        Step {
            report: Some(self.keep(look)),
            ..Step::default()
        }
    }

    /// Takes the size and place of the window into the geometry to save.
    fn keep(&mut self, look: Look) -> WindowGeometry {
        // A maximized window will return to its normal size and place.
        if !look.maximized {
            if let Some([width, height]) = look.size {
                self.normal.width = width;
                self.normal.height = height;
            }
            if look.place.is_some() {
                self.normal.place = look.place;
            }
        }
        WindowGeometry {
            maximized: look.maximized,
            ..self.normal
        }
    }
}

/// Whether two points or sizes differ by more than rounding.
fn far(a: [f32; 2], b: [f32; 2]) -> bool {
    (a[0] - b[0]).abs() > NEAR || (a[1] - b[1]).abs() > NEAR
}

/// Asks the system for the saved size and place of the window.
pub fn restore(builder: egui::ViewportBuilder, saved: &WindowGeometry) -> egui::ViewportBuilder {
    let mut builder = builder.with_inner_size([saved.width as f32, saved.height as f32]);
    if let Some(place) = saved.place {
        // The position is given in points; the window is put on its screen and then
        // placed with that screen's scale. A screen of another scale is set right by
        // the tracker.
        let points = 100.0 / place.scale as f32;
        builder = builder.with_position([place.x as f32 * points, place.y as f32 * points]);
    }
    if saved.maximized && !IN_POINTS {
        builder = builder.with_maximized(true);
    }
    builder
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: [f32; 2] = [1920.0, 1080.0];
    const FRAME: [f32; 2] = [1140.0, 800.0];

    fn place(x: i32, y: i32) -> WindowPlace {
        WindowPlace {
            x,
            y,
            screen_width: 1920,
            screen_height: 1080,
            scale: 100,
        }
    }

    fn saved(place: Option<WindowPlace>) -> WindowGeometry {
        WindowGeometry {
            width: 1120,
            height: 760,
            maximized: false,
            place,
        }
    }

    fn view(position: [f32; 2]) -> View {
        View {
            per_point: 1.0,
            scale: 1.0,
            size: [1120.0, 760.0],
            position: Some(position),
            outer_size: Some(FRAME),
            screen: Some(SCREEN),
            maximized: false,
            hidden: false,
        }
    }

    /// Feeds the same view until the tracker asks for something other than a frame.
    /// Returns that step and the time.
    fn run(tracker: &mut Tracker, view: &View, mut now: f64) -> (Step, f64) {
        for _ in 0..1000 {
            let step = tracker.step(view, now);
            if step.report.is_some() || step.move_to.is_some() || step.maximize {
                return (step, now);
            }
            match step.wake {
                Some(wake) => now += wake,
                None => return (step, now),
            }
        }
        panic!("the tracker keeps waiting");
    }

    #[test]
    fn a_window_without_a_saved_place_is_only_watched() {
        let mut tracker = Tracker::new(saved(None));
        assert_eq!(tracker.stage, Stage::Tracking);
        let (step, now) = run(&mut tracker, &view([40.0, 30.0]), 0.0);
        assert_eq!(step.move_to, None);
        assert_eq!(step.report, Some(saved(Some(place(40, 30)))));
        assert!((now - SETTLE).abs() < 1e-9);
    }

    #[test]
    fn a_window_in_its_place_is_left_alone() {
        let geometry = saved(Some(place(200, 100)));
        let mut tracker = Tracker::new(geometry);
        let (step, now) = run(&mut tracker, &view([200.0, 100.0]), 0.0);
        assert_eq!(step.move_to, None);
        assert_eq!(step.report, Some(geometry));
        assert!((STILL + SETTLE - 1e-9..PLACING_WAIT + SETTLE).contains(&now));
    }

    #[test]
    fn the_place_is_judged_only_when_the_window_stands_still() {
        let mut tracker = Tracker::new(saved(Some(place(200, 100))));
        // The window manager moves the window several times after it appears.
        let mut now = 0.0;
        for x in [0.0, 20.0, 60.0, 150.0] {
            let step = tracker.step(&view([x, 100.0]), now);
            assert_eq!(step.move_to, None, "moved at {x}");
            now += 0.1;
        }
        let (step, _) = run(&mut tracker, &view([150.0, 100.0]), now);
        assert_eq!(step.move_to, Some([200.0, 100.0]));
    }

    #[test]
    fn a_window_that_keeps_moving_is_judged_after_a_while() {
        let mut tracker = Tracker::new(saved(Some(place(200, 100))));
        let mut now = 0.0;
        let mut x = 0.0;
        loop {
            let step = tracker.step(&view([x, 100.0]), now);
            if let Some(target) = step.move_to {
                assert_eq!(target, [200.0, 100.0]);
                assert!(now >= PLACING_WAIT);
                break;
            }
            assert!(now < PLACING_WAIT + 0.1);
            x += 1.0;
            now += 0.1;
        }
    }

    #[test]
    fn a_window_out_of_place_is_moved_back_once() {
        let geometry = saved(Some(place(200, 100)));
        let mut tracker = Tracker::new(geometry);
        let (step, now) = run(&mut tracker, &view([180.0, 90.0]), 0.0);
        assert_eq!(step.move_to, Some([200.0, 100.0]));
        assert_eq!(tracker.stage, Stage::Moving);
        // The window manager does not let it go there: the tracker does not insist.
        let (step, _) = run(&mut tracker, &view([180.0, 90.0]), now);
        assert_eq!(step.move_to, None);
        assert_eq!(tracker.stage, Stage::Tracking);
        assert_eq!(step.report, Some(saved(Some(place(180, 90)))));
    }

    #[test]
    fn a_window_whose_screen_is_gone_is_centred_on_the_main_screen() {
        let mut tracker = Tracker::new(saved(Some(WindowPlace {
            screen_width: 2560,
            screen_height: 1440,
            ..place(2100, 100)
        })));
        let (step, now) = run(&mut tracker, &view([2100.0, 100.0]), 0.0);
        assert_eq!(step.move_to, Some([0.0, 0.0]));
        assert_eq!(tracker.stage, Stage::ToMainScreen);
        let (step, now) = run(&mut tracker, &view([0.0, 0.0]), now);
        assert_eq!(step.move_to, Some([390.0, 140.0]));
        assert_eq!(tracker.stage, Stage::Tracking);
        let (step, _) = run(&mut tracker, &view([390.0, 140.0]), now);
        assert_eq!(step.report, Some(saved(Some(place(390, 140)))));
    }

    #[test]
    fn a_frame_larger_than_the_screen_goes_to_its_corner() {
        let mut tracker = Tracker::new(saved(Some(WindowPlace {
            screen_width: 3840,
            screen_height: 2160,
            ..place(100, 100)
        })));
        let (step, now) = run(&mut tracker, &view([100.0, 100.0]), 0.0);
        assert_eq!(step.move_to, Some([0.0, 0.0]));
        let small = View {
            screen: Some([1024.0, 768.0]),
            ..view([0.0, 0.0])
        };
        let (step, _) = run(&mut tracker, &small, now);
        assert_eq!(step.move_to, Some([0.0, 0.0]));
    }

    #[test]
    fn a_window_without_a_position_is_not_placed() {
        // Wayland does not tell programs where their windows are.
        let mut tracker = Tracker::new(saved(Some(place(200, 100))));
        let wayland = View {
            position: None,
            outer_size: None,
            ..view([0.0, 0.0])
        };
        let (step, _) = run(&mut tracker, &wayland, 0.0);
        assert_eq!(step.move_to, None);
        // The size is still saved, and the old place is kept.
        assert_eq!(step.report, Some(saved(Some(place(200, 100)))));
    }

    #[test]
    fn changes_are_saved_once_they_settle() {
        let mut tracker = Tracker::new(saved(None));
        let mut now = 0.0;
        // Dragging the window: no report while it moves.
        for x in 0..20 {
            let step = tracker.step(&view([x as f32 * 10.0, 50.0]), now);
            assert_eq!(step.report, None);
            now += 0.05;
        }
        let resting = View {
            size: [1300.4, 820.6],
            ..view([190.0, 50.0])
        };
        let (step, then) = run(&mut tracker, &resting, now);
        assert!((then - now - SETTLE).abs() < 1e-9);
        let expected = WindowGeometry {
            width: 1300,
            height: 821,
            ..saved(Some(place(190, 50)))
        };
        assert_eq!(step.report, Some(expected));
        // The same look is not reported again.
        assert_eq!(tracker.step(&resting, then + 5.0), Step::default());
        assert_eq!(tracker.last(), None);
    }

    #[test]
    fn a_maximized_window_keeps_its_normal_size_and_place() {
        let mut tracker = Tracker::new(saved(Some(place(200, 100))));
        let (_, now) = run(&mut tracker, &view([200.0, 100.0]), 0.0);
        let (step, _) = run(&mut tracker, &maximized(), now);
        assert_eq!(
            step.report,
            Some(WindowGeometry {
                maximized: true,
                ..saved(Some(place(200, 100)))
            })
        );
    }

    fn maximized() -> View {
        View {
            size: [1920.0, 1040.0],
            outer_size: Some([1920.0, 1080.0]),
            maximized: true,
            ..view([0.0, 0.0])
        }
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn a_maximized_window_is_not_placed() {
        let geometry = WindowGeometry {
            maximized: true,
            ..saved(Some(place(200, 100)))
        };
        let mut tracker = Tracker::new(geometry);
        assert_eq!(tracker.stage, Stage::Maximizing);
        assert!(!tracker.normal.maximized);
        let (step, _) = run(&mut tracker, &maximized(), 0.0);
        assert_eq!(step.move_to, None);
        assert!(!step.maximize);
        assert_eq!(step.report, Some(geometry));
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn a_window_saved_maximized_is_maximized_once_shown() {
        // The window manager has shown the window in its normal size and place.
        let geometry = WindowGeometry {
            maximized: true,
            ..saved(Some(place(200, 100)))
        };
        let mut tracker = Tracker::new(geometry);
        let (step, now) = run(&mut tracker, &view([200.0, 100.0]), 0.0);
        assert!(step.maximize);
        assert_eq!(step.move_to, None);
        assert_eq!(tracker.stage, Stage::Tracking);
        let (step, _) = run(&mut tracker, &maximized(), now);
        assert!(!step.maximize);
        assert_eq!(step.report, Some(geometry));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn a_window_saved_maximized_is_placed_in_points() {
        // macOS does not tell programs whether their windows are zoomed.
        let mut tracker = Tracker::new(WindowGeometry {
            maximized: true,
            ..saved(Some(place(200, 100)))
        });
        assert_eq!(tracker.stage, Stage::Start);
        let (step, _) = run(&mut tracker, &view([0.0, 0.0]), 0.0);
        assert_eq!(step.move_to, Some([200.0, 100.0]));
        assert!(!step.maximize);
    }

    #[test]
    fn a_minimized_window_is_not_saved() {
        let mut tracker = Tracker::new(saved(None));
        let minimized = View {
            hidden: true,
            ..view([10.0, 10.0])
        };
        assert_eq!(tracker.step(&minimized, 0.0), Step::default());
        assert_eq!(tracker.step(&minimized, 10.0), Step::default());
        assert_eq!(tracker.last(), None);
    }

    #[test]
    fn a_change_not_yet_saved_is_given_when_the_window_closes() {
        let mut tracker = Tracker::new(saved(None));
        let step = tracker.step(&view([10.0, 20.0]), 0.0);
        assert_eq!(step.report, None);
        assert_eq!(tracker.last(), Some(saved(Some(place(10, 20)))));
        assert_eq!(tracker.last(), None);
    }

    #[test]
    fn nothing_is_given_when_the_window_closes_while_it_is_placed() {
        let mut tracker = Tracker::new(saved(Some(place(200, 100))));
        let step = tracker.step(&view([0.0, 0.0]), 0.0);
        assert_eq!(step.report, None);
        assert_eq!(tracker.last(), None);
    }

    #[test]
    fn sizes_and_places_out_of_bounds_are_not_saved() {
        let mut tracker = Tracker::new(saved(Some(place(200, 100))));
        let (_, now) = run(&mut tracker, &view([200.0, 100.0]), 0.0);
        let odd = View {
            size: [100.0, 100.0],
            position: Some([1.0e6, 0.0]),
            ..view([0.0, 0.0])
        };
        let (step, _) = run(&mut tracker, &odd, now);
        assert_eq!(step.report, Some(saved(Some(place(200, 100)))));
    }

    #[test]
    fn the_look_is_measured_in_desktop_units() {
        let dense = View {
            per_point: 1.5,
            scale: 1.5,
            size: [1000.0, 700.0],
            position: Some([-1919.6, 30.4]),
            outer_size: Some([1520.0, 1100.0]),
            screen: Some([2880.0, 1620.0]),
            maximized: false,
            hidden: false,
        };
        assert_eq!(
            Look::of(&dense),
            Look {
                maximized: false,
                size: Some([1000, 700]),
                place: Some(WindowPlace {
                    x: -1920,
                    y: 30,
                    screen_width: 2880,
                    screen_height: 1620,
                    scale: 150,
                }),
            }
        );
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn the_viewport_is_read_in_pixels() {
        let info = egui::ViewportInfo {
            native_pixels_per_point: Some(2.0),
            monitor_size: Some(egui::vec2(1280.0, 720.0)),
            outer_rect: Some(egui::Rect::from_min_size(
                egui::pos2(-50.0, 20.0),
                egui::vec2(600.0, 400.0),
            )),
            maximized: Some(true),
            minimized: Some(false),
            ..Default::default()
        };
        // With the interface zoomed in, a point is larger.
        let view = View::from_info(&info, 1.25, [750.0, 487.5]).unwrap();
        assert_eq!(view.per_point, 2.5);
        assert_eq!(view.scale, 2.0);
        assert_eq!(view.position, Some([-125.0, 50.0]));
        assert_eq!(view.outer_size, Some([1500.0, 1000.0]));
        assert_eq!(view.screen, Some([3200.0, 1800.0]));
        assert!(view.maximized && !view.hidden);
        assert_eq!(
            View::from_info(&egui::ViewportInfo::default(), 1.0, [1.0, 1.0]),
            None
        );
    }

    #[test]
    fn the_window_is_asked_for_in_points() {
        let geometry = WindowGeometry {
            width: 1300,
            height: 900,
            maximized: true,
            place: Some(WindowPlace {
                scale: 125,
                ..place(-500, 250)
            }),
        };
        let builder = restore(egui::ViewportBuilder::default(), &geometry);
        assert_eq!(builder.inner_size, Some(egui::vec2(1300.0, 900.0)));
        assert_eq!(builder.position, Some(egui::pos2(-400.0, 200.0)));
        assert_eq!(builder.maximized, (!IN_POINTS).then_some(true));

        let builder = restore(egui::ViewportBuilder::default(), &saved(None));
        assert_eq!(builder.inner_size, Some(egui::vec2(1120.0, 760.0)));
        assert_eq!(builder.position, None);
        assert_eq!(builder.maximized, None);
    }
}
