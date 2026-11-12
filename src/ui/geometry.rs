//! Геометрия доски: размещение в окне и пересчёт координат мыши в поля.
//!
//! Все координаты — в логических точках интерфейса; ось Y направлена вниз.
//! Размер клетки округляется до целого числа физических пикселей, поэтому границы
//! клеток остаются резкими при любом размере окна и плотности экрана.

use crate::board::Square;

/// Доля стороны доски, занимаемая рамкой с координатами (с каждой стороны).
pub const FRAME_FRACTION: f32 = 0.05;

/// Точка доски в экранных клетках: единица — сторона клетки, ось Y направлена вниз,
/// начало — левый верхний угол игрового поля. Целым `file` и `rank` соответствует
/// центр клетки. `flipped` — доска перевёрнута (внизу чёрные).
#[must_use]
pub fn screen_cell(file: f32, rank: f32, flipped: bool) -> (f32, f32) {
    let (column, row) = if flipped {
        (7.0 - file, rank)
    } else {
        (file, 7.0 - rank)
    };
    (column + 0.5, row + 0.5)
}

/// Размещение доски в прямоугольной области.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BoardLayout {
    /// Левый верхний угол доски вместе с рамкой.
    pub outer_x: f32,
    pub outer_y: f32,
    /// Сторона доски вместе с рамкой.
    pub outer_size: f32,
    /// Левый верхний угол игрового поля 8×8.
    pub field_x: f32,
    pub field_y: f32,
    /// Сторона клетки.
    pub cell: f32,
    /// Доска перевёрнута: внизу чёрные (первая горизонталь сверху, вертикаль `h` слева).
    pub flipped: bool,
}

impl BoardLayout {
    /// Вписывает квадратную доску в область `width × height` с левым верхним углом
    /// `(x, y)` и центрирует её. `pixels_per_point` — плотность экрана.
    #[must_use]
    pub fn fit(x: f32, y: f32, width: f32, height: f32, flipped: bool, pixels_per_point: f32) -> Self {
        let ppp = if pixels_per_point.is_finite() && pixels_per_point > 0.01 {
            pixels_per_point
        } else {
            1.0
        };
        let width = if width.is_finite() { width.max(0.0) } else { 0.0 };
        let height = if height.is_finite() { height.max(0.0) } else { 0.0 };
        let side_px = (width.min(height) * ppp).floor().max(8.0);
        let wanted_frame_px = (side_px * FRAME_FRACTION).round();
        let cell_px = ((side_px - 2.0 * wanted_frame_px) / 8.0).floor().max(1.0);
        let frame_px = ((side_px - 8.0 * cell_px) / 2.0).floor().max(0.0);
        let outer_px = 8.0 * cell_px + 2.0 * frame_px;
        let outer_x_px = ((x + (width - outer_px / ppp) / 2.0) * ppp).round();
        let outer_y_px = ((y + (height - outer_px / ppp) / 2.0) * ppp).round();
        Self {
            outer_x: outer_x_px / ppp,
            outer_y: outer_y_px / ppp,
            outer_size: outer_px / ppp,
            field_x: (outer_x_px + frame_px) / ppp,
            field_y: (outer_y_px + frame_px) / ppp,
            cell: cell_px / ppp,
            flipped,
        }
    }

    /// Сторона игрового поля.
    #[must_use]
    pub fn field_size(&self) -> f32 {
        8.0 * self.cell
    }

    /// Ширина рамки.
    #[must_use]
    pub fn frame(&self) -> f32 {
        (self.outer_size - self.field_size()) / 2.0
    }

    /// Вертикаль, изображённая в столбце `column` (считая слева от нуля).
    #[must_use]
    pub fn file_of_column(&self, column: u8) -> u8 {
        if self.flipped {
            7 - column.min(7)
        } else {
            column.min(7)
        }
    }

    /// Горизонталь, изображённая в строке `row` (считая сверху от нуля).
    #[must_use]
    pub fn rank_of_row(&self, row: u8) -> u8 {
        if self.flipped {
            row.min(7)
        } else {
            7 - row.min(7)
        }
    }

    /// Клетка (вертикаль, горизонталь) под точкой; `None` вне игрового поля.
    #[must_use]
    pub fn cell_at(&self, px: f32, py: f32) -> Option<(u8, u8)> {
        if self.cell <= 0.0 || !px.is_finite() || !py.is_finite() {
            return None;
        }
        let fx = (px - self.field_x) / self.cell;
        let fy = (py - self.field_y) / self.cell;
        if !(0.0..8.0).contains(&fx) || !(0.0..8.0).contains(&fy) {
            return None;
        }
        let column = (fx.floor() as u8).min(7);
        let row = (fy.floor() as u8).min(7);
        Some((self.file_of_column(column), self.rank_of_row(row)))
    }

    /// Игровое (тёмное) поле под точкой; `None` для светлых клеток и точек вне поля.
    #[must_use]
    pub fn square_at(&self, px: f32, py: f32) -> Option<Square> {
        let (file, rank) = self.cell_at(px, py)?;
        Square::from_file_rank(file, rank)
    }

    /// Левый верхний угол клетки.
    #[must_use]
    pub fn cell_origin(&self, file: u8, rank: u8) -> (f32, f32) {
        let (cx, cy) = self.point_at(f32::from(file.min(7)), f32::from(rank.min(7)));
        (cx - self.cell / 2.0, cy - self.cell / 2.0)
    }

    /// Центр поля.
    #[must_use]
    pub fn square_center(&self, sq: Square) -> (f32, f32) {
        self.point_at(f32::from(sq.file()), f32::from(sq.rank()))
    }

    /// Точка, соответствующая дробным координатам доски (центр клетки `(file, rank)`
    /// при целых значениях). Используется для движущихся шашек.
    #[must_use]
    pub fn point_at(&self, file: f32, rank: f32) -> (f32, f32) {
        self.cell_to_point(screen_cell(file, rank, self.flipped))
    }

    /// Точка окна, соответствующая точке в экранных клетках (см. [`screen_cell`]).
    #[must_use]
    pub fn cell_to_point(&self, cell: (f32, f32)) -> (f32, f32) {
        (
            self.field_x + cell.0 * self.cell,
            self.field_y + cell.1 * self.cell,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_is_square_centered_and_inside_the_area() {
        for &(w, h) in &[
            (800.0f32, 600.0f32),
            (600.0, 800.0),
            (333.0, 333.0),
            (1920.0, 1080.0),
            (97.0, 55.0),
        ] {
            for &ppp in &[1.0f32, 1.25, 1.5, 2.0, 3.0] {
                let l = BoardLayout::fit(10.0, 20.0, w, h, false, ppp);
                assert!(l.outer_size <= w.min(h) + 0.01);
                assert!(l.outer_x >= 10.0 - 1.0 && l.outer_y >= 20.0 - 1.0);
                assert!(l.outer_x + l.outer_size <= 10.0 + w + 1.0);
                assert!(l.outer_y + l.outer_size <= 20.0 + h + 1.0);
                // Доска отцентрована с точностью до пикселя.
                let cx = l.outer_x + l.outer_size / 2.0;
                let cy = l.outer_y + l.outer_size / 2.0;
                assert!((cx - (10.0 + w / 2.0)).abs() <= 1.0, "{w}x{h}@{ppp}");
                assert!((cy - (20.0 + h / 2.0)).abs() <= 1.0, "{w}x{h}@{ppp}");
                // Клетка занимает целое число физических пикселей.
                let cell_px = l.cell * ppp;
                assert!(
                    (cell_px - cell_px.round()).abs() < 1e-3,
                    "{w}x{h}@{ppp}: {cell_px}"
                );
                assert!((l.frame() * 2.0 + l.field_size() - l.outer_size).abs() < 1e-3);
            }
        }
    }

    #[test]
    fn screen_cells_follow_the_orientation() {
        // Белые внизу: a1 — левая нижняя клетка, h8 — правая верхняя.
        assert_eq!(screen_cell(0.0, 0.0, false), (0.5, 7.5));
        assert_eq!(screen_cell(7.0, 7.0, false), (7.5, 0.5));
        // Доска перевёрнута: a1 справа вверху.
        assert_eq!(screen_cell(0.0, 0.0, true), (7.5, 0.5));
        assert_eq!(screen_cell(7.0, 7.0, true), (0.5, 7.5));
        for flipped in [false, true] {
            let layout = BoardLayout::fit(13.0, 7.0, 640.0, 480.0, flipped, 1.5);
            for sq in Square::all() {
                let cell = screen_cell(f32::from(sq.file()), f32::from(sq.rank()), flipped);
                assert_eq!(layout.cell_to_point(cell), layout.square_center(sq));
            }
        }
    }

    #[test]
    fn degenerate_areas_do_not_panic() {
        for &(w, h, ppp) in &[
            (0.0f32, 0.0f32, 1.0f32),
            (-5.0, 10.0, 1.0),
            (f32::NAN, 100.0, 1.0),
            (100.0, f32::INFINITY, 0.0),
            (1.0, 1.0, f32::NAN),
        ] {
            let l = BoardLayout::fit(0.0, 0.0, w, h, true, ppp);
            assert!(l.cell > 0.0);
            let _ = l.square_at(0.0, 0.0);
            let _ = l.square_at(f32::NAN, 0.0);
        }
    }
}
