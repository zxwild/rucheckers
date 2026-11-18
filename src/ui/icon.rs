//! The application icon and the files it is kept in.
//!
//! The icon is the white man of the board on a dark square, drawn by the same code as
//! the pieces. The window takes its icon straight from [`render_icon`]. Two places need
//! the icon as a file before the program runs:
//!
//! * the resources of the Windows executable — `assets/icons/rucheckers.ico`, shown by
//!   Explorer, the Start menu and pinned shortcuts;
//! * the desktop entry on Linux — `assets/icons/rucheckers.png`, shown by application
//!   menus and, under Wayland, by the dock and the window switcher.
//!
//! Both files are written by `cargo run --example icons`; a test checks that they still
//! match the drawing code. The file formats are simple enough to be written here
//! without a dependency: the pixels are stored uncompressed.
//!
//! macOS draws application icons on a grid of its own: a rounded shape smaller than the
//! image, with a transparent margin and a shadow, so that the icon stands in line with
//! the others in the Dock and in Finder. [`render_macos_icon`] draws that variant; the
//! application bundle gets it as a set of images ([`ICONSET`]) that `iconutil` packs
//! into an `.icns` file. The bundle is built by `tools/build-macos-app.sh`, so these
//! images are not kept in the repository.

use crate::board::{Color, Kind, Piece};
use crate::ui::sprite::render_piece;

/// Sides of the images in the `.ico` file: the sizes Windows asks for at the display
/// scales of 100 to 200 %, and the large one for Explorer.
pub const ICO_SIZES: [usize; 8] = [16, 20, 24, 32, 40, 48, 64, 256];
/// Side of the image in the `.png` file.
pub const PNG_SIZE: usize = 256;
/// The largest side an `.ico` file can describe.
const ICO_MAX_SIZE: usize = 256;

/// Anti-aliasing of the background edge: samples per pixel along each axis.
const SAMPLES: usize = 3;
/// The colour of the dark square under the piece.
const SQUARE: [f32; 3] = [0.63, 0.44, 0.25];

/// A square picture of the icon: RGBA without premultiplication, row by row from the top.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct IconImage {
    /// Side of the image in pixels.
    pub size: usize,
    /// The pixels: four bytes each (R, G, B, A).
    pub rgba: Vec<u8>,
}

impl IconImage {
    /// The pixel in the column `x` and the row `y`.
    ///
    /// # Panics
    ///
    /// Panics if the pixel is outside the image.
    #[must_use]
    pub fn pixel(&self, x: usize, y: usize) -> [u8; 4] {
        assert!(
            x < self.size && y < self.size,
            "pixel ({x}, {y}) is outside the icon"
        );
        let i = 4 * (y * self.size + x);
        [self.rgba[i], self.rgba[i + 1], self.rgba[i + 2], self.rgba[i + 3]]
    }
}

/// Whether a point (in fractions of the icon side) is inside the rounded background square.
fn inside_square(x: f32, y: f32) -> bool {
    let (u, v) = (x * 2.0 - 1.0, y * 2.0 - 1.0);
    let corner = 0.22;
    let (qx, qy) = (
        (u.abs() - (0.94 - corner)).max(0.0),
        (v.abs() - (0.94 - corner)).max(0.0),
    );
    qx.hypot(qy) <= corner
}

/// Draws the icon with a side of `size` pixels: a white piece on a dark square with
/// rounded corners, the same piece as on the board.
#[must_use]
pub fn render_icon(size: usize) -> IconImage {
    let piece = render_piece(Piece::new(Color::White, Kind::Man), size, true);
    let mut rgba = Vec::with_capacity(size * size * 4);
    let step = 1.0 / (size * SAMPLES) as f32;
    for py in 0..size {
        for px in 0..size {
            let mut inside = 0;
            for sy in 0..SAMPLES {
                for sx in 0..SAMPLES {
                    let x = ((px * SAMPLES + sx) as f32 + 0.5) * step;
                    let y = ((py * SAMPLES + sy) as f32 + 0.5) * step;
                    inside += usize::from(inside_square(x, y));
                }
            }
            let background = inside as f32 / (SAMPLES * SAMPLES) as f32;
            // The piece (with premultiplied alpha) is laid over the background.
            let top = piece.pixel(px, py);
            let top_alpha = f32::from(top[3]) / 255.0;
            let alpha = top_alpha + background * (1.0 - top_alpha);
            for channel in 0..3 {
                let premultiplied =
                    f32::from(top[channel]) / 255.0 + SQUARE[channel] * background * (1.0 - top_alpha);
                // The icon is stored without premultiplication.
                let value = if alpha > 0.0 { premultiplied / alpha } else { 0.0 };
                rgba.push((value * 255.0).round().clamp(0.0, 255.0) as u8);
            }
            rgba.push((alpha * 255.0).round().clamp(0.0, 255.0) as u8);
        }
    }
    IconImage { size, rgba }
}

/// The images of a macOS icon set (`rucheckers.iconset`): the names `iconutil` expects
/// and the sides in pixels. Every size is there twice, for ordinary and for Retina
/// screens (`@2x`).
pub const ICONSET: [(&str, usize); 10] = [
    ("icon_16x16.png", 16),
    ("icon_16x16@2x.png", 32),
    ("icon_32x32.png", 32),
    ("icon_32x32@2x.png", 64),
    ("icon_128x128.png", 128),
    ("icon_128x128@2x.png", 256),
    ("icon_256x256.png", 256),
    ("icon_256x256@2x.png", 512),
    ("icon_512x512.png", 512),
    ("icon_512x512@2x.png", 1024),
];

/// Apple's grid of application icons: on an image of 1024 pixels the shape takes 824,
/// the margin around it is left for the shadow.
const MACOS_GRID: usize = 1024;
const MACOS_SHAPE: usize = 824;
/// The corner of the rounded rectangle that casts the shadow, in halves of the side of
/// the shape. It follows the outline of the shape (see [`inside_macos_shape`]).
const MACOS_CORNER: f32 = 0.45;
/// The shadow under the shape: its opacity, then its offset downwards and its blur in
/// pixels of the 1024-pixel grid.
const MACOS_SHADOW: (f32, f32, f32) = (0.3, 10.0, 12.0);

/// The transparent margin around the shape of a macOS icon with a side of `size` pixels.
const fn macos_margin(size: usize) -> usize {
    // The margin of the grid scaled to the image, rounded to the nearest pixel.
    (size * (MACOS_GRID - MACOS_SHAPE) + MACOS_GRID) / (2 * MACOS_GRID)
}

/// Whether a point is inside the shape of a macOS icon. The coordinates run from −1 to
/// 1 across the shape.
///
/// The outline is a superellipse, |u|⁵ + |v|⁵ ≤ 1: its corners bend smoothly into the
/// sides, as the corners of the icons of the system do.
fn inside_macos_shape(u: f32, v: f32) -> bool {
    let (u, v) = (u.abs(), v.abs());
    let (u2, v2) = (u * u, v * v);
    u2 * u2 * u + v2 * v2 * v <= 1.0
}

/// The signed distance from a point to the rounded rectangle close to the shape of a
/// macOS icon, in the coordinates of [`inside_macos_shape`]; negative inside.
fn macos_shape_distance(u: f32, v: f32) -> f32 {
    let qx = u.abs() - (1.0 - MACOS_CORNER);
    let qy = v.abs() - (1.0 - MACOS_CORNER);
    qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - MACOS_CORNER
}

/// Rises smoothly from 0 at `low` to 1 at `high`.
fn smoothstep(low: f32, high: f32, x: f32) -> f32 {
    let t = ((x - low) / (high - low)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Draws the macOS variant of the icon with a side of `size` pixels: the same piece and
/// square as [`render_icon`], laid out on Apple's grid — the square has the outline of
/// the icons of the system, leaves a transparent margin and casts a soft shadow.
#[must_use]
pub fn render_macos_icon(size: usize) -> IconImage {
    if size == 0 {
        return IconImage {
            size,
            rgba: Vec::new(),
        };
    }
    let margin = macos_margin(size);
    let inner = size - 2 * margin;
    let piece = render_piece(Piece::new(Color::White, Kind::Man), inner, true);
    // Pixels in a unit of the shape coordinates, and the centre of the image.
    let half = inner as f32 / 2.0;
    let centre = size as f32 / 2.0;
    let scale = size as f32 / MACOS_GRID as f32;
    let (opacity, offset, blur) = MACOS_SHADOW;
    let (offset, blur) = (offset * scale, (blur * scale).max(0.5));

    let mut rgba = Vec::with_capacity(size * size * 4);
    for py in 0..size {
        for px in 0..size {
            let mut inside = 0;
            for sy in 0..SAMPLES {
                for sx in 0..SAMPLES {
                    let x = px as f32 + (sx as f32 + 0.5) / SAMPLES as f32;
                    let y = py as f32 + (sy as f32 + 0.5) / SAMPLES as f32;
                    inside += usize::from(inside_macos_shape((x - centre) / half, (y - centre) / half));
                }
            }
            let coverage = inside as f32 / (SAMPLES * SAMPLES) as f32;
            // The shadow is the shape moved down and blurred, black.
            let distance = half
                * macos_shape_distance(
                    (px as f32 + 0.5 - centre) / half,
                    (py as f32 + 0.5 - centre - offset) / half,
                );
            let shadow = opacity * (1.0 - smoothstep(-blur, blur, distance));
            // The square over the shadow, the piece (premultiplied) over both.
            let under = coverage + shadow * (1.0 - coverage);
            let top = if px >= margin && py >= margin {
                piece.pixel(px - margin, py - margin)
            } else {
                [0; 4]
            };
            let top_alpha = f32::from(top[3]) / 255.0;
            let alpha = top_alpha + under * (1.0 - top_alpha);
            for channel in 0..3 {
                let premultiplied =
                    f32::from(top[channel]) / 255.0 + SQUARE[channel] * coverage * (1.0 - top_alpha);
                let value = if alpha > 0.0 { premultiplied / alpha } else { 0.0 };
                rgba.push((value * 255.0).round().clamp(0.0, 255.0) as u8);
            }
            rgba.push((alpha * 255.0).round().clamp(0.0, 255.0) as u8);
        }
    }
    IconImage { size, rgba }
}

/// The icon as a Windows `.ico` file with an image of every size of [`ICO_SIZES`].
#[must_use]
pub fn ico_file() -> Vec<u8> {
    let images: Vec<IconImage> = ICO_SIZES.iter().map(|&size| render_icon(size)).collect();
    encode_ico(&images)
}

/// The icon as a `.png` file with a side of [`PNG_SIZE`] pixels.
#[must_use]
pub fn png_file() -> Vec<u8> {
    encode_png(&render_icon(PNG_SIZE))
}

fn push_u16(out: &mut Vec<u8>, value: usize) {
    out.extend_from_slice(&(value as u16).to_le_bytes());
}

fn push_u32(out: &mut Vec<u8>, value: usize) {
    out.extend_from_slice(&(value as u32).to_le_bytes());
}

/// Bytes in one row of the transparency mask of an icon: one bit per pixel, padded to
/// a multiple of four bytes.
const fn mask_row_bytes(size: usize) -> usize {
    size.div_ceil(32) * 4
}

/// Writes images into the `.ico` container.
///
/// Every image is stored the classic way, as an uncompressed 32-bit bitmap followed by
/// a one-bit transparency mask: every resource compiler and every version of Windows
/// reads this form.
///
/// # Panics
///
/// Panics if an image is empty or larger than 256 pixels, the limit of the format.
#[must_use]
pub fn encode_ico(images: &[IconImage]) -> Vec<u8> {
    const DIRECTORY_HEADER: usize = 6;
    const DIRECTORY_ENTRY: usize = 16;
    const BITMAP_HEADER: usize = 40;

    let bitmap_bytes = |size: usize| BITMAP_HEADER + size * size * 4 + size * mask_row_bytes(size);
    let mut out = Vec::new();
    // The directory: a reserved word, the type (1 is an icon) and the number of images.
    push_u16(&mut out, 0);
    push_u16(&mut out, 1);
    push_u16(&mut out, images.len());
    let mut offset = DIRECTORY_HEADER + DIRECTORY_ENTRY * images.len();
    for image in images {
        let size = image.size;
        assert!(
            (1..=ICO_MAX_SIZE).contains(&size),
            "an icon of {size} pixels does not fit the format"
        );
        // A side of 256 pixels is written as zero.
        let side = (size % ICO_MAX_SIZE) as u8;
        out.extend_from_slice(&[side, side, 0, 0]);
        push_u16(&mut out, 1); // colour planes
        push_u16(&mut out, 32); // bits per pixel
        push_u32(&mut out, bitmap_bytes(size));
        push_u32(&mut out, offset);
        offset += bitmap_bytes(size);
    }
    for image in images {
        let size = image.size;
        // BITMAPINFOHEADER. The height covers the colours and the mask together.
        push_u32(&mut out, BITMAP_HEADER);
        push_u32(&mut out, size);
        push_u32(&mut out, 2 * size);
        push_u16(&mut out, 1); // colour planes
        push_u16(&mut out, 32); // bits per pixel
        push_u32(&mut out, 0); // no compression
        push_u32(&mut out, size * size * 4 + size * mask_row_bytes(size));
        out.extend_from_slice(&[0; 16]); // resolution and palette: unused
                                         // The rows go from the bottom up; the channels are blue, green, red, alpha.
        for y in (0..size).rev() {
            for x in 0..size {
                let [red, green, blue, alpha] = image.pixel(x, y);
                out.extend_from_slice(&[blue, green, red, alpha]);
            }
        }
        // The mask: a set bit marks a fully transparent pixel.
        for y in (0..size).rev() {
            let mut row = vec![0u8; mask_row_bytes(size)];
            for x in 0..size {
                if image.pixel(x, y)[3] == 0 {
                    row[x / 8] |= 0x80 >> (x % 8);
                }
            }
            out.extend_from_slice(&row);
        }
    }
    out
}

/// The CRC-32 checksum of the PNG format (the polynomial of zlib and Ethernet).
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// The Adler-32 checksum of the zlib format.
fn adler32(bytes: &[u8]) -> u32 {
    const MODULUS: u32 = 65_521;
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in bytes {
        a = (a + u32::from(byte)) % MODULUS;
        b = (b + a) % MODULUS;
    }
    (b << 16) | a
}

/// Wraps data into a zlib stream of stored (uncompressed) blocks.
fn zlib_stored(data: &[u8]) -> Vec<u8> {
    /// The longest stored block the deflate format allows.
    const BLOCK: usize = 0xFFFF;
    let mut out = vec![0x78, 0x01];
    let mut blocks = data.chunks(BLOCK).peekable();
    if blocks.peek().is_none() {
        // Even empty data needs one final block.
        out.extend_from_slice(&[1, 0, 0, 0xFF, 0xFF]);
    }
    while let Some(block) = blocks.next() {
        out.push(u8::from(blocks.peek().is_none()));
        let length = block.len() as u16;
        out.extend_from_slice(&length.to_le_bytes());
        out.extend_from_slice(&(!length).to_le_bytes());
        out.extend_from_slice(block);
    }
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

/// Appends a PNG chunk: the length, the type, the data and the checksum of the last two.
fn push_chunk(out: &mut Vec<u8>, kind: [u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let start = out.len();
    out.extend_from_slice(&kind);
    out.extend_from_slice(data);
    let checksum = crc32(&out[start..]);
    out.extend_from_slice(&checksum.to_be_bytes());
}

/// Writes an image as a PNG file: eight bits per channel, RGBA, no interlacing.
#[must_use]
pub fn encode_png(image: &IconImage) -> Vec<u8> {
    let size = image.size;
    let mut header = Vec::with_capacity(13);
    header.extend_from_slice(&(size as u32).to_be_bytes());
    header.extend_from_slice(&(size as u32).to_be_bytes());
    // Bit depth 8, colour type 6 (RGBA), then the only compression and filter methods
    // of the format, and no interlacing.
    header.extend_from_slice(&[8, 6, 0, 0, 0]);

    // Every row starts with its filter type; 0 leaves the bytes as they are.
    let mut rows = Vec::with_capacity(size * (size * 4 + 1));
    for row in image.rgba.chunks(size.max(1) * 4) {
        rows.push(0);
        rows.extend_from_slice(row);
    }

    let mut out = vec![0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1A, b'\n'];
    push_chunk(&mut out, *b"IHDR", &header);
    push_chunk(&mut out, *b"IDAT", &zlib_stored(&rows));
    push_chunk(&mut out, *b"IEND", &[]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u16_at(bytes: &[u8], at: usize) -> usize {
        usize::from(u16::from_le_bytes([bytes[at], bytes[at + 1]]))
    }

    fn u32_at(bytes: &[u8], at: usize) -> usize {
        u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]) as usize
    }

    fn be32_at(bytes: &[u8], at: usize) -> u32 {
        u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
    }

    /// Reads the images back from an `.ico` file written by [`encode_ico`], checking
    /// the layout on the way.
    fn decode_ico(bytes: &[u8]) -> Vec<IconImage> {
        assert_eq!(u16_at(bytes, 0), 0, "reserved word");
        assert_eq!(u16_at(bytes, 2), 1, "the file is an icon");
        let count = u16_at(bytes, 4);
        let mut end_of_previous = 6 + 16 * count;
        let mut images = Vec::new();
        for index in 0..count {
            let entry = 6 + 16 * index;
            let size = match bytes[entry] {
                0 => 256,
                side => usize::from(side),
            };
            assert_eq!(bytes[entry + 1], bytes[entry], "the image is square");
            assert_eq!(&bytes[entry + 2..entry + 4], [0, 0], "no palette");
            assert_eq!(u16_at(bytes, entry + 4), 1, "planes");
            assert_eq!(u16_at(bytes, entry + 6), 32, "bits per pixel");
            let length = u32_at(bytes, entry + 8);
            let offset = u32_at(bytes, entry + 12);
            assert_eq!(
                offset, end_of_previous,
                "the images follow each other without gaps"
            );
            end_of_previous = offset + length;

            let bitmap = &bytes[offset..offset + length];
            assert_eq!(u32_at(bitmap, 0), 40, "BITMAPINFOHEADER");
            assert_eq!(u32_at(bitmap, 4), size, "width");
            assert_eq!(u32_at(bitmap, 8), 2 * size, "the height counts the mask too");
            assert_eq!(u16_at(bitmap, 12), 1, "planes");
            assert_eq!(u16_at(bitmap, 14), 32, "bits per pixel");
            assert_eq!(u32_at(bitmap, 16), 0, "no compression");
            let mask_row = mask_row_bytes(size);
            assert_eq!(length, 40 + size * size * 4 + size * mask_row);
            assert_eq!(u32_at(bitmap, 20), length - 40, "size of the pixel data");

            let mut rgba = vec![0u8; size * size * 4];
            for y in 0..size {
                // Rows are stored from the bottom up.
                let row = 40 + (size - 1 - y) * size * 4;
                let mask = 40 + size * size * 4 + (size - 1 - y) * mask_row;
                for x in 0..size {
                    let [blue, green, red, alpha]: [u8; 4] =
                        bitmap[row + 4 * x..row + 4 * x + 4].try_into().unwrap();
                    rgba[4 * (y * size + x)..4 * (y * size + x) + 4]
                        .copy_from_slice(&[red, green, blue, alpha]);
                    let masked = bitmap[mask + x / 8] & (0x80 >> (x % 8)) != 0;
                    assert_eq!(
                        masked,
                        alpha == 0,
                        "mask bit of ({x}, {y}) in the {size}-pixel image"
                    );
                }
            }
            images.push(IconImage { size, rgba });
        }
        assert_eq!(end_of_previous, bytes.len(), "nothing follows the last image");
        images
    }

    /// Reads the image back from a `.png` file written by [`encode_png`], checking
    /// the checksums on the way.
    fn decode_png(bytes: &[u8]) -> IconImage {
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n", "signature");
        let mut chunks = Vec::new();
        let mut at = 8;
        while at < bytes.len() {
            let length = be32_at(bytes, at) as usize;
            let body = &bytes[at + 4..at + 8 + length];
            assert_eq!(
                be32_at(bytes, at + 8 + length),
                crc32(body),
                "checksum of a chunk"
            );
            chunks.push((&body[..4], &body[4..]));
            at += 12 + length;
        }
        let kinds: Vec<&[u8]> = chunks.iter().map(|(kind, _)| *kind).collect();
        assert_eq!(kinds, [b"IHDR", b"IDAT", b"IEND"]);
        assert_eq!(chunks[2].1, [], "the end chunk is empty");

        let header = chunks[0].1;
        let size = be32_at(header, 0) as usize;
        assert_eq!(be32_at(header, 4) as usize, size, "the image is square");
        assert_eq!(&header[8..], [8, 6, 0, 0, 0], "8 bits, RGBA, not interlaced");

        // The zlib stream: a header, stored blocks, the checksum of the data.
        let stream = chunks[1].1;
        assert_eq!(&stream[..2], [0x78, 0x01]);
        assert_eq!(
            (usize::from(stream[0]) * 256 + usize::from(stream[1])) % 31,
            0,
            "zlib header check"
        );
        let mut data = Vec::new();
        let mut at = 2;
        loop {
            let last = stream[at];
            assert!(last <= 1, "a stored block");
            let length = u16_at(stream, at + 1);
            assert_eq!(
                u16_at(stream, at + 3),
                !length & 0xFFFF,
                "complement of the length"
            );
            data.extend_from_slice(&stream[at + 5..at + 5 + length]);
            at += 5 + length;
            if last == 1 {
                break;
            }
        }
        assert_eq!(be32_at(stream, at), adler32(&data), "checksum of the pixels");
        assert_eq!(at + 4, stream.len(), "nothing follows the checksum");

        assert_eq!(data.len(), size * (size * 4 + 1));
        let mut rgba = Vec::with_capacity(size * size * 4);
        for row in data.chunks(size * 4 + 1) {
            assert_eq!(row[0], 0, "rows are not filtered");
            rgba.extend_from_slice(&row[1..]);
        }
        IconImage { size, rgba }
    }

    /// A picture with a different value in every channel of every pixel.
    fn pattern(size: usize) -> IconImage {
        let mut rgba = Vec::new();
        for y in 0..size {
            for x in 0..size {
                let alpha = if (x + y) % 3 == 0 {
                    0
                } else {
                    255 - (x * 7 % 200) as u8
                };
                rgba.extend_from_slice(&[(x * 5) as u8, (y * 9) as u8, (x * y) as u8, alpha]);
            }
        }
        IconImage { size, rgba }
    }

    #[test]
    fn checksums_match_reference_values() {
        // Reference values of the zlib library.
        assert_eq!(crc32(b""), 0);
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b"IEND"), 0xAE42_6082);
        assert_eq!(adler32(b""), 1);
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
        // Sums of a long input stay within the modulus.
        assert_eq!(adler32(&vec![0xFF; 100_000]), 0x149A_302C);
    }

    #[test]
    fn ico_keeps_every_pixel_of_every_size() {
        let images: Vec<IconImage> = [1, 16, 20, 33, 48, 255, 256].map(pattern).to_vec();
        let file = encode_ico(&images);
        assert_eq!(decode_ico(&file), images);
        assert_eq!(decode_ico(&encode_ico(&[])), []);
    }

    #[test]
    fn png_keeps_every_pixel() {
        // 200 pixels need several stored blocks; 1 pixel and no pixels need one.
        for size in [0, 1, 7, 128, 200] {
            let image = pattern(size);
            let file = encode_png(&image);
            assert_eq!(decode_png(&file), image, "{size}");
        }
        // Exactly one full block, and one byte more.
        for length in [0xFFFF, 0x1_0000] {
            let stream = zlib_stored(&vec![7; length]);
            assert_eq!(
                stream.len(),
                2 + length + 5 * length.div_ceil(0xFFFF) + 4,
                "{length}"
            );
        }
    }

    #[test]
    fn the_icon_is_a_light_piece_on_a_dark_rounded_square() {
        for size in ICO_SIZES {
            let icon = render_icon(size);
            assert_eq!(icon.rgba.len(), size * size * 4);
            // The corners are cut off, the middle of every side belongs to the square.
            for (x, y) in [(0, 0), (size - 1, 0), (0, size - 1), (size - 1, size - 1)] {
                assert_eq!(
                    icon.pixel(x, y)[3],
                    0,
                    "corner ({x}, {y}) of the {size}-pixel icon"
                );
            }
            // The edge of the square runs 3 % of the side away from the edge of the image.
            let near = size * 6 / 100 + 1;
            let far = size - 1 - near;
            for (x, y) in [
                (size / 2, near),
                (near, size / 2),
                (size / 2, far),
                (far, size / 2),
            ] {
                assert_eq!(
                    icon.pixel(x, y)[3],
                    255,
                    "side ({x}, {y}) of the {size}-pixel icon"
                );
            }
            // The piece in the centre is lighter than the square, which shows between
            // the piece and the rounded corner.
            let brightness =
                |[red, green, blue, _]: [u8; 4]| u32::from(red) + u32::from(green) + u32::from(blue);
            let piece = brightness(icon.pixel(size / 2, size / 2));
            let uncovered = size * 14 / 100;
            assert_eq!(icon.pixel(uncovered, uncovered)[3], 255, "{size}");
            let square = brightness(icon.pixel(uncovered, uncovered));
            assert!(piece > square + 150, "{size}: piece {piece}, square {square}");
        }
        assert_eq!(render_icon(0).rgba, []);
    }

    #[test]
    fn the_icon_set_has_every_size_macos_asks_for() {
        for (name, size) in ICONSET {
            let (side, retina) = match name.strip_suffix("@2x.png") {
                Some(rest) => (rest, 2),
                None => (name.strip_suffix(".png").unwrap(), 1),
            };
            let side = side.strip_prefix("icon_").unwrap();
            let (width, height) = side.split_once('x').unwrap();
            assert_eq!(width, height, "{name}");
            assert_eq!(width.parse::<usize>().unwrap() * retina, size, "{name}");
        }
        let sides: Vec<usize> = ICONSET.iter().map(|(_, size)| *size).collect();
        assert_eq!(sides, [16, 32, 32, 64, 128, 256, 256, 512, 512, 1024]);
    }

    #[test]
    fn the_macos_icon_follows_apples_grid() {
        // 100 of 1024 pixels on each side, as on Apple's template.
        assert_eq!(macos_margin(1024), 100);
        assert_eq!(macos_margin(512), 50);
        let brightness = |[red, green, blue, _]: [u8; 4]| u32::from(red) + u32::from(green) + u32::from(blue);
        for (size, margin) in [(16, 2), (32, 3), (64, 6), (128, 13), (256, 25)] {
            assert_eq!(macos_margin(size), margin);
            let icon = render_macos_icon(size);
            assert_eq!(icon.rgba.len(), size * size * 4);
            // The edges of the image are transparent: the margin and the shadow fade out
            // before them.
            for i in 0..size {
                for (x, y) in [(i, 0), (0, i), (i, size - 1), (size - 1, i)] {
                    assert_eq!(icon.pixel(x, y)[3], 0, "edge ({x}, {y}) of {size} pixels");
                }
            }
            // The shape starts right after the margin, at the middle of the top side…
            let middle = size / 2;
            assert_eq!(icon.pixel(middle, margin)[3], 255, "{size}");
            assert_eq!(icon.pixel(middle, margin - 1)[3], 0, "{size}");
            assert_eq!(icon.pixel(margin, middle)[3], 255, "{size}");
            // Beside the shape only the blurred edge of the shadow shows.
            assert!(icon.pixel(margin - 1, middle)[3] < 40, "{size}");
            // …and its corners are rounded off.
            assert!(icon.pixel(margin, margin)[3] < 128, "{size}");
            // The piece in the centre is lighter than the square around it.
            let inner = size - 2 * margin;
            let uncovered = margin + inner * 12 / 100;
            assert_eq!(icon.pixel(uncovered, uncovered)[3], 255, "{size}");
            let piece = brightness(icon.pixel(middle, middle));
            let square = brightness(icon.pixel(uncovered, uncovered));
            assert!(piece > square + 150, "{size}: piece {piece}, square {square}");
        }
        // The shadow falls below the shape, not above it.
        let icon = render_macos_icon(256);
        let below = icon.pixel(128, 256 - 25 + 2)[3];
        let above = icon.pixel(128, 25 - 2)[3];
        assert!(below > above + 20, "below {below}, above {above}");
        assert_eq!(render_macos_icon(0).rgba, []);
    }

    /// The largest difference between the channels of two images of the same size.
    fn largest_difference(a: &IconImage, b: &IconImage) -> u8 {
        assert_eq!(a.size, b.size);
        a.rgba
            .iter()
            .zip(&b.rgba)
            .map(|(x, y)| x.abs_diff(*y))
            .max()
            .unwrap_or(0)
    }

    /// The icon files in `assets/icons/` are made by `cargo run --example icons`. If
    /// this test fails, the drawing of the icon or of a piece has changed: run the
    /// example again and commit the files.
    ///
    /// A difference of a couple of levels is allowed: the mathematical functions of the
    /// system libraries round differently on different systems.
    #[test]
    fn the_icon_files_match_the_drawing_code() {
        const TOLERANCE: u8 = 2;
        let ico = decode_ico(include_bytes!("../../assets/icons/rucheckers.ico"));
        let sizes: Vec<usize> = ico.iter().map(|image| image.size).collect();
        assert_eq!(sizes, ICO_SIZES);
        for image in &ico {
            let difference = largest_difference(image, &render_icon(image.size));
            assert!(
                difference <= TOLERANCE,
                "{} pixels: differs by {difference}",
                image.size
            );
        }
        let png = decode_png(include_bytes!("../../assets/icons/rucheckers.png"));
        assert_eq!(png.size, PNG_SIZE);
        let difference = largest_difference(&png, &render_icon(PNG_SIZE));
        assert!(difference <= TOLERANCE, "the PNG file differs by {difference}");
    }
}
