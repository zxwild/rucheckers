//! Native material viewer: `cargo run --example board_designs`.
//! `--variant 1 --pieces`, `--detail`, or `--capture work/board-designs-native`.
//! Captures the actual egui/OpenGL framebuffer to portable PPM files (no extra crate).

// Reuse the game's piece drawing without duplicating its sprites and shading.
#[allow(dead_code)]
#[path = "../src/gui/board.rs"]
mod board;
#[path = "../src/gui/board_materials.rs"]
mod materials;
#[allow(dead_code)]
#[path = "../src/gui/theme.rs"]
mod theme;

use eframe::egui::{self, pos2, vec2, Align2, FontId, Key, Rect};
use materials::{BoardArt, MATERIALS};
use rucheckers::games::{Color, Kind, Piece};
use rucheckers::ui::geometry::BoardLayout;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Empty,
    Pieces,
    Detail,
}

struct Viewer {
    style: usize,
    view: View,
    art: BoardArt,
    sprites: Option<board::PieceSprites>,
    capture: Option<PathBuf>,
    capture_one: bool,
    capture_stage: usize,
    settled_frames: usize,
    waiting: bool,
}

impl Viewer {
    fn from_args() -> Result<Self, String> {
        let mut viewer = Self {
            style: 0,
            view: View::Empty,
            art: BoardArt::default(),
            sprites: None,
            capture: None,
            capture_one: false,
            capture_stage: 0,
            settled_frames: 0,
            waiting: false,
        };
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--variant" => {
                    let number: usize = args
                        .next()
                        .ok_or("--variant needs 1–12")?
                        .parse()
                        .map_err(|_| "--variant needs 1–12")?;
                    if !(1..=12).contains(&number) {
                        return Err("--variant needs 1–12".into());
                    }
                    viewer.style = number - 1;
                }
                "--pieces" => viewer.view = View::Pieces,
                "--detail" => {
                    viewer.view = View::Detail;
                }
                "--capture" => {
                    viewer.capture = Some(args.next().ok_or("--capture needs a directory")?.into());
                }
                "--capture-one" => {
                    viewer.capture = Some(args.next().ok_or("--capture-one needs a directory")?.into());
                    viewer.capture_one = true;
                }
                _ => {
                    return Err(format!(
                    "Unknown argument {arg}. Use --variant 1–12, --pieces, --detail, --capture DIRECTORY."
                ))
                }
            }
        }
        if let Some(path) = &viewer.capture {
            std::fs::create_dir_all(path).map_err(|e| e.to_string())?;
            if !viewer.capture_one {
                viewer.style = 0;
            }
            viewer.view = View::Empty;
        }
        Ok(viewer)
    }

    fn capture_event(&mut self, ctx: &egui::Context) {
        let Some(directory) = self.capture.clone() else {
            return;
        };
        let screenshot = ctx.input(|input| {
            input.events.iter().find_map(|event| {
                if let egui::Event::Screenshot { image, .. } = event {
                    Some(image.clone())
                } else {
                    None
                }
            })
        });
        if let Some(image) = screenshot {
            let suffix = match self.capture_stage % 3 {
                0 => "",
                1 => "-with-pieces",
                _ => "-detail",
            };
            let path = directory.join(format!("{:02}{suffix}.ppm", self.style + 1));
            if let Err(error) = save_ppm(&path, &image) {
                eprintln!("{}: {error}", path.display());
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
            println!(
                "Captured {} ({} × {})",
                path.display(),
                image.size[0],
                image.size[1]
            );
            self.capture_stage += 1;
            self.waiting = false;
            self.settled_frames = 0;
            if self.capture_stage >= if self.capture_one { 3 } else { MATERIALS.len() * 3 } {
                self.capture = None;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                if !self.capture_one {
                    self.style = self.capture_stage / 3;
                }
                self.view = match self.capture_stage % 3 {
                    0 => View::Empty,
                    1 => View::Pieces,
                    _ => View::Detail,
                };
            }
        }
    }
}

impl Viewer {
    fn draw_pieces(
        &mut self,
        ctx: &egui::Context,
        painter: &egui::Painter,
        layout: &BoardLayout,
        display: Rect,
    ) {
        if self.view != View::Empty {
            let size = (layout.cell * ctx.pixels_per_point()).round() as usize;
            if self.sprites.as_ref().map(board::PieceSprites::size_px) != Some(size) {
                self.sprites = Some(board::PieceSprites::new(ctx, size));
            }
            let sprites = self.sprites.as_ref().expect("sprites were initialized");
            for row in [0u8, 1, 6, 7] {
                for column in 0..8u8 {
                    if (column + row) % 2 == 0 || (self.view == View::Detail && (row != 1 || column != 0)) {
                        continue;
                    }
                    let center = pos2(
                        layout.field_x + (f32::from(column) + 0.5) * layout.cell,
                        layout.field_y + (f32::from(row) + 0.5) * layout.cell,
                    );
                    if !display.contains(center) {
                        continue;
                    }
                    let piece = Piece::new(if row < 2 { Color::Black } else { Color::White }, Kind::Man);
                    board::draw_piece(painter, sprites, center, layout.cell, piece, 1.0, false);
                }
            }
        }
    }
}

fn save_ppm(path: &Path, image: &egui::ColorImage) -> std::io::Result<()> {
    let mut writer = BufWriter::new(File::create(path)?);
    writeln!(writer, "P6\n{} {}\n255", image.size[0], image.size[1])?;
    for color in &image.pixels {
        writer.write_all(&[color.r(), color.g(), color.b()])?;
    }
    writer.flush()
}

impl eframe::App for Viewer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.capture_event(ctx);
        if self.capture.is_none() {
            ctx.input(|input| {
                if input.key_pressed(Key::ArrowRight) {
                    self.style = (self.style + 1) % MATERIALS.len();
                }
                if input.key_pressed(Key::ArrowLeft) {
                    self.style = (self.style + MATERIALS.len() - 1) % MATERIALS.len();
                }
                if input.key_pressed(Key::Space) {
                    self.view = if self.view == View::Empty {
                        View::Pieces
                    } else {
                        View::Empty
                    };
                }
                if input.key_pressed(Key::Z) {
                    self.view = if self.view == View::Detail {
                        View::Pieces
                    } else {
                        View::Detail
                    };
                }
            });
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(theme::BACKGROUND))
            .show(ctx, |ui| {
                let area = ui.max_rect();
                let painter = ui.painter();
                let material = MATERIALS[self.style];
                painter.text(
                    pos2(area.left() + 28.0, area.top() + 33.0),
                    Align2::LEFT_CENTER,
                    format!("{:02} — {}", self.style + 1, material.name),
                    FontId::proportional(27.0),
                    theme::TEXT,
                );
                painter.text(
                    pos2(area.left() + 28.0, area.top() + 69.0),
                    Align2::LEFT_CENTER,
                    material.description,
                    FontId::proportional(16.0),
                    theme::TEXT_WEAK,
                );
                let preview = Rect::from_min_max(
                    pos2(area.left() + 25.0, area.top() + 100.0),
                    pos2(area.right() - 25.0, area.bottom() - 61.0),
                );
                let side = preview.width().min(preview.height());
                let top_left = preview.min + vec2((preview.width() - side) * 0.5, 0.0);
                let display = Rect::from_min_size(top_left, vec2(side, side));
                let board_size = if self.view == View::Detail {
                    side * 3.5
                } else {
                    side - 10.0
                };
                let layout = BoardLayout::fit(
                    display.left() + 5.0,
                    display.top() + 5.0,
                    board_size,
                    board_size,
                    false,
                    ctx.pixels_per_point(),
                );
                let clipped = painter.with_clip_rect(display);
                self.art
                    .draw(&clipped, &layout, self.style, ctx.pixels_per_point());
                self.draw_pieces(ctx, &clipped, &layout, display);
                let mode = if self.view == View::Detail {
                    "Крупный план: волокна, фаски и угловой стык"
                } else if self.view != View::Empty {
                    "С шашками приложения"
                } else {
                    "Полная доска без шашек"
                };
                painter.text(
                    pos2(area.left() + 28.0, area.bottom() - 37.0),
                    Align2::LEFT_CENTER,
                    mode,
                    FontId::proportional(15.0),
                    theme::TEXT_WEAK,
                );
                painter.text(
                    pos2(area.right() - 28.0, area.bottom() - 37.0),
                    Align2::RIGHT_CENTER,
                    "Rust / egui · программная отрисовка",
                    FontId::proportional(14.0),
                    theme::TEXT_WEAK,
                );
            });
        if self.capture.is_some() {
            self.settled_frames += 1;
            if self.settled_frames >= 4 && !self.waiting {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
                self.waiting = true;
            }
            ctx.request_repaint();
        }
    }
}

fn main() -> eframe::Result<()> {
    let viewer = match Viewer::from_args() {
        Ok(viewer) => viewer,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("rucheckers — материалы доски")
            .with_inner_size([1120.0, 1240.0])
            .with_min_inner_size([500.0, 620.0]),
        ..Default::default()
    };
    eframe::run_native(
        "rucheckers-board-designs",
        options,
        Box::new(move |cc| {
            theme::apply(&cc.egui_ctx);
            Ok(Box::new(viewer))
        }),
    )
}
